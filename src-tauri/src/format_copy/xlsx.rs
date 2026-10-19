//! xlsx → `XML Spreadsheet`（SpreadsheetML 2003）+ `text/html` + `text/plain`（TSV）。
//!
//! 只转换活动工作表的已用区域；公式写缓存值，不写 `ss:Formula`；
//! 样式完整展开，不使用 `ss:Parent`（WPS 会把父样式解析错，见预研）。

use std::{collections::HashMap, io::Cursor};

use umya_spreadsheet::{
    drawing::Theme, BorderStyleValues, CellRawValue, HorizontalAlignmentValues, PatternValues,
    UnderlineValues, VerticalAlignmentValues, Worksheet,
};

use super::{copy_failed, too_large, FormatSet, MAX_XLSX_CELLS};
use crate::error::AppError;

pub fn convert(bytes: &[u8]) -> Result<FormatSet, AppError> {
    // 损坏或加密的文件可能让解析库返回错误，个别情况下会 panic；两者都按转换失败处理。
    let book = std::panic::catch_unwind(|| umya_spreadsheet::reader::xlsx::read_reader(Cursor::new(bytes), true))
        .map_err(|_| unreadable())?
        .map_err(|_| unreadable())?;
    let sheet = book.active_sheet();
    let (cols, rows) = sheet.highest_column_and_row();
    if u64::from(cols) * u64::from(rows) > MAX_XLSX_CELLS {
        return Err(too_large());
    }
    let grid = Grid::read(sheet, book.theme(), cols, rows);
    let mut set = FormatSet::default();
    set.push("XML Spreadsheet", grid.to_spreadsheet_ml().into_bytes());
    set.push("text/html", grid.to_html().into_bytes());
    let tsv = grid.to_tsv().into_bytes();
    set.push("text/plain", tsv.clone());
    set.push("UTF8_STRING", tsv);
    if grid.formulas > 0 {
        set.notes.push("公式已转为数值。".into());
    }
    if grid.formulas_without_value > 0 {
        set.notes
            .push("部分公式没有缓存值，对应单元格已留空。".into());
    }
    Ok(set)
}

fn unreadable() -> AppError {
    copy_failed("无法读取该表格（文件损坏或已加密）。请用外部软件打开。")
}

// ---------------------------------------------------------------- 中间模型

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Line {
    css: &'static str,
    width_px: u8,
    ss_style: &'static str,
    ss_weight: u8,
    color: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
struct Style {
    font_name: String,
    /// 字号 × 100，避免浮点参与哈希。
    font_size: u32,
    bold: bool,
    italic: bool,
    underline: &'static str,
    strike: bool,
    font_color: Option<String>,
    fill: Option<String>,
    horizontal: &'static str,
    vertical: &'static str,
    wrap: bool,
    /// left、right、top、bottom。
    borders: [Option<Line>; 4],
    number_format: Option<String>,
}

#[derive(Debug, Clone)]
enum Value {
    Empty,
    Text(String),
    Number(String),
    DateTime(String),
    Boolean(bool),
}

#[derive(Debug, Clone)]
struct Cell {
    value: Value,
    /// 应用了数字格式的显示文本（HTML、TSV 使用）。
    display: String,
    style: usize,
}

struct Grid {
    sheet_name: String,
    cols: u32,
    rows: u32,
    styles: Vec<Style>,
    /// 行优先；合并区域内被覆盖的格子为 `None`。
    cells: Vec<Vec<Option<Cell>>>,
    /// 合并锚点 (列, 行) → (横向跨度, 纵向跨度)（0 起，即 MergeAcross / MergeDown）。
    merges: HashMap<(u32, u32), (u32, u32)>,
    col_widths: Vec<f64>,
    row_heights: Vec<Option<f64>>,
    formulas: u32,
    formulas_without_value: u32,
}

const DEFAULT_COL_CHARS: f64 = 8.43;

impl Grid {
    fn read(sheet: &Worksheet, theme: &Theme, cols: u32, rows: u32) -> Self {
        let mut merges = HashMap::new();
        let mut covered = std::collections::HashSet::new();
        for range in sheet.merge_cells() {
            let (Some(c1), Some(r1), Some(c2), Some(r2)) = (
                range.coordinate_start_col(),
                range.coordinate_start_row(),
                range.coordinate_end_col(),
                range.coordinate_end_row(),
            ) else {
                continue;
            };
            let (c1, r1, c2, r2) = (c1.num(), r1.num(), c2.num(), r2.num());
            if c2 < c1 || r2 < r1 || (c1 == c2 && r1 == r2) {
                continue;
            }
            merges.insert((c1, r1), (c2 - c1, r2 - r1));
            for r in r1..=r2 {
                for c in c1..=c2 {
                    if (c, r) != (c1, r1) {
                        covered.insert((c, r));
                    }
                }
            }
        }

        let mut styles: Vec<Style> = vec![Style::default()];
        let mut style_ids: HashMap<Style, usize> = HashMap::from([(Style::default(), 0)]);
        let mut formulas = 0;
        let mut formulas_without_value = 0;
        let mut cells = Vec::with_capacity(rows as usize);
        for r in 1..=rows {
            let mut row = Vec::with_capacity(cols as usize);
            for c in 1..=cols {
                if covered.contains(&(c, r)) {
                    row.push(None);
                    continue;
                }
                let Some(cell) = sheet.cell((c, r)) else {
                    row.push(Some(Cell {
                        value: Value::Empty,
                        display: String::new(),
                        style: 0,
                    }));
                    continue;
                };
                let style = resolve_style(cell.style(), theme);
                let is_date = style
                    .number_format
                    .as_deref()
                    .is_some_and(is_date_format);
                let style_id = *style_ids.entry(style.clone()).or_insert_with(|| {
                    styles.push(style);
                    styles.len() - 1
                });
                let value = match cell.raw_value() {
                    CellRawValue::Numeric(n) if n.is_finite() => {
                        if is_date {
                            Value::DateTime(serial_to_iso(*n))
                        } else {
                            Value::Number(n.to_string())
                        }
                    }
                    CellRawValue::Numeric(_) | CellRawValue::Empty => Value::Empty,
                    CellRawValue::Bool(b) => Value::Boolean(*b),
                    CellRawValue::String(_)
                    | CellRawValue::RichText(_)
                    | CellRawValue::Lazy(_)
                    | CellRawValue::Error(_) => Value::Text(cell.value().into_owned()),
                };
                if cell.is_formula() {
                    if matches!(value, Value::Empty) {
                        formulas_without_value += 1;
                    } else {
                        formulas += 1;
                    }
                }
                let display = match &value {
                    Value::Empty => String::new(),
                    _ => cell.formatted_value(),
                };
                row.push(Some(Cell {
                    value,
                    display,
                    style: style_id,
                }));
            }
            cells.push(row);
        }

        let col_widths = (1..=cols)
            .map(|c| {
                sheet
                    .get_column_dimension_by_number(&c)
                    .map(|col| col.width())
                    .filter(|w| *w > 0.0)
                    .unwrap_or(DEFAULT_COL_CHARS)
            })
            .collect();
        let row_heights = (1..=rows)
            .map(|r| {
                sheet
                    .get_row_dimension(&r)
                    .filter(|row| row.custom_height() && row.height() > 0.0)
                    .map(|row| row.height())
            })
            .collect();
        Self {
            sheet_name: sheet.name().to_owned(),
            cols,
            rows,
            styles,
            cells,
            merges,
            col_widths,
            row_heights,
            formulas,
            formulas_without_value,
        }
    }

    // ------------------------------------------------------------ SpreadsheetML

    fn to_spreadsheet_ml(&self) -> String {
        let mut out = String::new();
        out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        out.push_str("<?mso-application progid=\"Excel.Sheet\"?>\n");
        out.push_str("<Workbook xmlns=\"urn:schemas-microsoft-com:office:spreadsheet\" xmlns:o=\"urn:schemas-microsoft-com:office:office\" xmlns:x=\"urn:schemas-microsoft-com:office:excel\" xmlns:ss=\"urn:schemas-microsoft-com:office:spreadsheet\" xmlns:html=\"http://www.w3.org/TR/REC-html40\">\n<Styles>\n");
        for (id, style) in self.styles.iter().enumerate() {
            out.push_str(&style_xml(id, style));
        }
        out.push_str("</Styles>\n");
        out.push_str(&format!(
            "<Worksheet ss:Name=\"{}\">\n<Table ss:ExpandedColumnCount=\"{}\" ss:ExpandedRowCount=\"{}\" x:FullColumns=\"1\" x:FullRows=\"1\">\n",
            xml_escape(&self.sheet_name),
            self.cols,
            self.rows
        ));
        for width in &self.col_widths {
            out.push_str(&format!(
                "<Column ss:AutoFitWidth=\"0\" ss:Width=\"{:.2}\"/>\n",
                chars_to_px(*width) * 0.75
            ));
        }
        for (row_index, row) in self.cells.iter().enumerate() {
            let r = row_index as u32 + 1;
            match self.row_heights[row_index] {
                Some(height) => out.push_str(&format!("<Row ss:AutoFitHeight=\"0\" ss:Height=\"{height:.2}\">")),
                None => out.push_str("<Row>"),
            }
            let mut previous_end = 0u32; // 上一个已输出单元格占用到的列号
            for (col_index, cell) in row.iter().enumerate() {
                let c = col_index as u32 + 1;
                let Some(cell) = cell else { continue };
                let merge = self.merges.get(&(c, r));
                if matches!(cell.value, Value::Empty) && cell.style == 0 && merge.is_none() {
                    continue;
                }
                let mut attrs = String::new();
                if c != previous_end + 1 {
                    attrs.push_str(&format!(" ss:Index=\"{c}\""));
                }
                if cell.style != 0 {
                    attrs.push_str(&format!(" ss:StyleID=\"s{}\"", cell.style));
                }
                if let Some((across, down)) = merge {
                    if *across > 0 {
                        attrs.push_str(&format!(" ss:MergeAcross=\"{across}\""));
                    }
                    if *down > 0 {
                        attrs.push_str(&format!(" ss:MergeDown=\"{down}\""));
                    }
                }
                let data = match &cell.value {
                    Value::Empty => String::new(),
                    Value::Text(text) => data_xml("String", &xml_escape(text)),
                    Value::Number(n) => data_xml("Number", n),
                    Value::DateTime(iso) => data_xml("DateTime", iso),
                    Value::Boolean(b) => data_xml("Boolean", if *b { "1" } else { "0" }),
                };
                out.push_str(&format!("<Cell{attrs}>{data}</Cell>"));
                previous_end = c + merge.map_or(0, |(across, _)| *across);
            }
            out.push_str("</Row>\n");
        }
        out.push_str("</Table>\n</Worksheet>\n</Workbook>\n");
        out
    }

    // ------------------------------------------------------------ HTML

    fn to_html(&self) -> String {
        let mut out = String::from("<meta charset=\"utf-8\"><!--StartFragment--><table style=\"border-collapse:collapse\"><colgroup>");
        for width in &self.col_widths {
            out.push_str(&format!("<col width=\"{}\">", chars_to_px(*width).round() as u32));
        }
        out.push_str("</colgroup>");
        for (row_index, row) in self.cells.iter().enumerate() {
            let r = row_index as u32 + 1;
            match self.row_heights[row_index] {
                Some(height) => out.push_str(&format!("<tr style=\"height:{height:.2}pt\">")),
                None => out.push_str("<tr>"),
            }
            for (col_index, cell) in row.iter().enumerate() {
                let c = col_index as u32 + 1;
                let Some(cell) = cell else { continue };
                let mut attrs = String::new();
                if let Some((across, down)) = self.merges.get(&(c, r)) {
                    if *across > 0 {
                        attrs.push_str(&format!(" colspan=\"{}\"", across + 1));
                    }
                    if *down > 0 {
                        attrs.push_str(&format!(" rowspan=\"{}\"", down + 1));
                    }
                }
                let css = css_for(&self.styles[cell.style]);
                if !css.is_empty() {
                    attrs.push_str(&format!(" style=\"{}\"", html_attr_escape(&css)));
                }
                let text = html_escape(&cell.display).replace("\r\n", "<br>").replace(['\n', '\r'], "<br>");
                out.push_str(&format!("<td{attrs}>{text}</td>"));
            }
            out.push_str("</tr>");
        }
        out.push_str("</table><!--EndFragment-->");
        out
    }

    // ------------------------------------------------------------ TSV

    fn to_tsv(&self) -> String {
        let mut out = String::new();
        for row in &self.cells {
            let fields: Vec<String> = row
                .iter()
                .map(|cell| cell.as_ref().map_or(String::new(), |c| tsv_field(&c.display)))
                .collect();
            out.push_str(&fields.join("\t"));
            out.push_str("\r\n");
        }
        out
    }
}

/// Excel 规则：含制表符、换行或双引号的字段加双引号，内部双引号翻倍。
fn tsv_field(value: &str) -> String {
    if value.contains(['\t', '\n', '\r', '"']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}

// ---------------------------------------------------------------- 样式解析

fn resolve_style(style: &umya_spreadsheet::Style, theme: &Theme) -> Style {
    let mut out = Style::default();
    if let Some(font) = style.font() {
        out.font_name = font.name().to_owned();
        out.font_size = (font.size() * 100.0).round().max(0.0) as u32;
        out.bold = font.bold();
        out.italic = font.italic();
        // `Font::underline()` 在文件未声明下划线时也返回 "single"，所以直接看枚举。
        out.underline = match font.font_underline().val() {
            UnderlineValues::None => "",
            UnderlineValues::Double | UnderlineValues::DoubleAccounting => "Double",
            _ => "Single",
        };
        out.strike = font.strikethrough();
        let color = font.color();
        if !color.automatic() {
            out.font_color = hex_color(&color.argb_with_theme(theme));
        }
    }
    if let Some(pattern) = style.fill().and_then(|fill| fill.pattern_fill()) {
        if matches!(pattern.pattern_type(), PatternValues::Solid) {
            out.fill = pattern
                .foreground_color()
                .and_then(|color| hex_color(&color.argb_with_theme(theme)));
        }
    }
    if let Some(alignment) = style.alignment() {
        out.horizontal = match alignment.horizontal() {
            HorizontalAlignmentValues::Left => "Left",
            HorizontalAlignmentValues::Center => "Center",
            HorizontalAlignmentValues::Right => "Right",
            HorizontalAlignmentValues::Justify => "Justify",
            HorizontalAlignmentValues::Distributed => "Distributed",
            HorizontalAlignmentValues::Fill => "Fill",
            HorizontalAlignmentValues::CenterContinuous => "CenterAcrossSelection",
            HorizontalAlignmentValues::General => "",
        };
        out.vertical = match alignment.vertical() {
            VerticalAlignmentValues::Top => "Top",
            VerticalAlignmentValues::Center => "Center",
            VerticalAlignmentValues::Bottom => "Bottom",
            VerticalAlignmentValues::Justify => "Justify",
            VerticalAlignmentValues::Distributed => "Distributed",
        };
        out.wrap = alignment.wrap_text();
    }
    if let Some(borders) = style.borders() {
        let sides = [
            borders.left_border(),
            borders.right_border(),
            borders.top_border(),
            borders.bottom_border(),
        ];
        for (slot, border) in out.borders.iter_mut().zip(sides) {
            let color = border
                .color()
                .and_then(|color| hex_color(&color.argb_with_theme(theme)))
                .unwrap_or_else(|| "000000".to_owned());
            *slot = line_for(border.style(), color);
        }
    }
    if let Some(format) = style.number_format() {
        let code = format.format_code();
        if !code.is_empty() && !code.eq_ignore_ascii_case("general") {
            out.number_format = Some(code.to_owned());
        }
    }
    out
}

/// "FFRRGGBB" / "RRGGBB" → "RRGGBB"（大写）。
fn hex_color(argb: &str) -> Option<String> {
    let hex = match argb.len() {
        8 => &argb[2..],
        6 => argb,
        _ => return None,
    };
    hex.chars()
        .all(|ch| ch.is_ascii_hexdigit())
        .then(|| hex.to_ascii_uppercase())
}

fn line_for(style: &BorderStyleValues, color: String) -> Option<Line> {
    use BorderStyleValues as B;
    let (css, width_px, ss_style, ss_weight) = match style {
        B::None => return None,
        B::Hair => ("solid", 1, "Continuous", 0),
        B::Thin => ("solid", 1, "Continuous", 1),
        B::Medium => ("solid", 2, "Continuous", 2),
        B::Thick => ("solid", 3, "Continuous", 3),
        B::Dashed => ("dashed", 1, "Dash", 1),
        B::MediumDashed => ("dashed", 2, "Dash", 2),
        B::Dotted => ("dotted", 1, "Dot", 1),
        B::DashDot => ("dashed", 1, "DashDot", 1),
        B::MediumDashDot => ("dashed", 2, "DashDot", 2),
        B::DashDotDot => ("dashed", 1, "DashDotDot", 1),
        B::MediumDashDotDot => ("dashed", 2, "DashDotDot", 2),
        B::SlantDashDot => ("dashed", 2, "SlantDashDot", 2),
        B::Double => ("double", 3, "Double", 3),
    };
    Some(Line {
        css,
        width_px,
        ss_style,
        ss_weight,
        color,
    })
}

const SIDES: [&str; 4] = ["Left", "Right", "Top", "Bottom"];

fn style_xml(id: usize, style: &Style) -> String {
    let mut out = if id == 0 {
        "<Style ss:ID=\"Default\" ss:Name=\"Normal\">".to_owned()
    } else {
        format!("<Style ss:ID=\"s{id}\">")
    };
    if !style.horizontal.is_empty() || !style.vertical.is_empty() || style.wrap {
        out.push_str("<Alignment");
        if !style.horizontal.is_empty() {
            out.push_str(&format!(" ss:Horizontal=\"{}\"", style.horizontal));
        }
        if !style.vertical.is_empty() {
            out.push_str(&format!(" ss:Vertical=\"{}\"", style.vertical));
        }
        if style.wrap {
            out.push_str(" ss:WrapText=\"1\"");
        }
        out.push_str("/>");
    }
    if style.borders.iter().any(Option::is_some) {
        out.push_str("<Borders>");
        for (side, line) in SIDES.iter().zip(&style.borders) {
            if let Some(line) = line {
                out.push_str(&format!(
                    "<Border ss:Position=\"{side}\" ss:LineStyle=\"{}\" ss:Weight=\"{}\" ss:Color=\"#{}\"/>",
                    line.ss_style, line.ss_weight, line.color
                ));
            }
        }
        out.push_str("</Borders>");
    }
    if !style.font_name.is_empty()
        || style.font_size > 0
        || style.bold
        || style.italic
        || !style.underline.is_empty()
        || style.strike
        || style.font_color.is_some()
    {
        out.push_str("<Font");
        if !style.font_name.is_empty() {
            out.push_str(&format!(" ss:FontName=\"{}\"", xml_escape(&style.font_name)));
        }
        if style.font_size > 0 {
            out.push_str(&format!(" ss:Size=\"{}\"", f64::from(style.font_size) / 100.0));
        }
        if let Some(color) = &style.font_color {
            out.push_str(&format!(" ss:Color=\"#{color}\""));
        }
        if style.bold {
            out.push_str(" ss:Bold=\"1\"");
        }
        if style.italic {
            out.push_str(" ss:Italic=\"1\"");
        }
        if !style.underline.is_empty() {
            out.push_str(&format!(" ss:Underline=\"{}\"", style.underline));
        }
        if style.strike {
            out.push_str(" ss:StrikeThrough=\"1\"");
        }
        out.push_str("/>");
    }
    if let Some(fill) = &style.fill {
        out.push_str(&format!("<Interior ss:Color=\"#{fill}\" ss:Pattern=\"Solid\"/>"));
    }
    if let Some(format) = &style.number_format {
        out.push_str(&format!("<NumberFormat ss:Format=\"{}\"/>", xml_escape(format)));
    }
    out.push_str("</Style>\n");
    out
}

fn css_for(style: &Style) -> String {
    let mut parts: Vec<String> = Vec::new();
    if !style.font_name.is_empty() {
        parts.push(format!(
            "font-family:'{}'",
            style.font_name.replace(['\'', '"', '\\', ';'], "")
        ));
    }
    if style.font_size > 0 {
        parts.push(format!("font-size:{}pt", f64::from(style.font_size) / 100.0));
    }
    if style.bold {
        parts.push("font-weight:bold".into());
    }
    if style.italic {
        parts.push("font-style:italic".into());
    }
    let mut decorations = Vec::new();
    if !style.underline.is_empty() {
        decorations.push("underline");
    }
    if style.strike {
        decorations.push("line-through");
    }
    if !decorations.is_empty() {
        parts.push(format!("text-decoration:{}", decorations.join(" ")));
    }
    if let Some(color) = &style.font_color {
        parts.push(format!("color:#{color}"));
    }
    if let Some(fill) = &style.fill {
        parts.push(format!("background-color:#{fill}"));
    }
    let align = match style.horizontal {
        "Left" => Some("left"),
        "Center" | "CenterAcrossSelection" => Some("center"),
        "Right" => Some("right"),
        "Justify" | "Distributed" => Some("justify"),
        _ => None,
    };
    if let Some(align) = align {
        parts.push(format!("text-align:{align}"));
    }
    match style.vertical {
        "Top" => parts.push("vertical-align:top".into()),
        "Center" => parts.push("vertical-align:middle".into()),
        "Bottom" => parts.push("vertical-align:bottom".into()),
        _ => {}
    }
    parts.push(
        if style.wrap {
            "white-space:pre-wrap"
        } else {
            "white-space:nowrap"
        }
        .into(),
    );
    for (side, line) in ["left", "right", "top", "bottom"].iter().zip(&style.borders) {
        if let Some(line) = line {
            parts.push(format!(
                "border-{side}:{}px {} #{}",
                line.width_px, line.css, line.color
            ));
        }
    }
    parts.join(";")
}

// ---------------------------------------------------------------- 工具

/// Excel 列宽（字符数）→ 像素。
fn chars_to_px(chars: f64) -> f64 {
    (chars * 7.0 + 5.0).trunc()
}

/// 数字格式是否是日期/时间格式：去掉引号、方括号与转义字符后含 y、m、d、h、s。
fn is_date_format(code: &str) -> bool {
    let mut chars = code.chars();
    let mut found = false;
    while let Some(ch) = chars.next() {
        match ch {
            '"' => {
                for inner in chars.by_ref() {
                    if inner == '"' {
                        break;
                    }
                }
            }
            '[' => {
                for inner in chars.by_ref() {
                    if inner == ']' {
                        break;
                    }
                }
            }
            '\\' | '_' | '*' => {
                chars.next();
            }
            'y' | 'Y' | 'm' | 'M' | 'd' | 'D' | 'h' | 'H' | 's' | 'S' => found = true,
            ';' => break, // 只看第一段
            _ => {}
        }
    }
    found
}

/// Excel 序列值（1900 系统，含 1900 闰年错误）→ `YYYY-MM-DDTHH:MM:SS.mmm`。
fn serial_to_iso(serial: f64) -> String {
    use chrono::{Duration, NaiveDate};
    let mut days = serial.floor();
    let mut millis = ((serial - days) * 86_400_000.0).round() as i64;
    if millis >= 86_400_000 {
        millis -= 86_400_000;
        days += 1.0;
    }
    let days = days as i64;
    let base = if days < 60 {
        NaiveDate::from_ymd_opt(1899, 12, 31)
    } else {
        NaiveDate::from_ymd_opt(1899, 12, 30)
    }
    .expect("static date");
    let date = base
        .checked_add_signed(Duration::days(days.min(2_958_465)))
        .unwrap_or(base);
    let time = Duration::milliseconds(millis);
    let (hours, minutes, seconds, ms) = (
        time.num_hours(),
        time.num_minutes() % 60,
        time.num_seconds() % 60,
        time.num_milliseconds() % 1000,
    );
    format!("{date}T{hours:02}:{minutes:02}:{seconds:02}.{ms:03}")
}

fn data_xml(kind: &str, body: &str) -> String {
    format!("<Data ss:Type=\"{kind}\">{body}</Data>")
}

/// XML 1.0 不允许的控制字符直接丢弃。
fn xml_escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            '\t' | '\n' | '\r' => out.push(ch),
            c if (c as u32) < 0x20 => {}
            c => out.push(c),
        }
    }
    out
}

fn html_escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            c => out.push(c),
        }
    }
    out
}

fn html_attr_escape(value: &str) -> String {
    html_escape(value).replace('"', "&quot;")
}

#[cfg(test)]
mod tests;
