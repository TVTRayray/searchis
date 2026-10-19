use super::*;
use umya_spreadsheet::{Border, HorizontalAlignmentValues, PatternValues};

fn to_bytes(book: &umya_spreadsheet::Workbook) -> Vec<u8> {
    let mut buffer = Cursor::new(Vec::new());
    umya_spreadsheet::writer::xlsx::write_writer(book, &mut buffer).unwrap();
    buffer.into_inner()
}

fn get<'a>(set: &'a FormatSet, name: &str) -> String {
    String::from_utf8(set.formats.iter().find(|(n, _)| n == name).unwrap().1.clone()).unwrap()
}

/// 覆盖预研样本（test2.xml）里 WPS 已验证可还原的特性：合并、字体、填充、边框、对齐、
/// 数字格式、日期、列宽、行高、公式缓存值。
fn sample() -> Vec<u8> {
    let mut book = umya_spreadsheet::new_file();
    let sheet = book.sheet_mut(0).unwrap();
    sheet.set_name("报销");
    sheet.cell_mut("A1").set_value("标题 <&>");
    {
        let style = sheet.style_mut("A1");
        style.font_mut().set_bold(true).set_italic(true).set_size(16.0);
        style.font_mut().set_name("Noto Sans");
        style.font_mut().color_mut().set_argb_str("FFFF0000");
        style
            .fill_mut()
            .pattern_fill_mut()
            .set_pattern_type(PatternValues::Solid)
            .foreground_color_mut()
            .set_argb_str("FFFFFF00");
        let alignment = style.alignment_mut();
        alignment.set_horizontal(HorizontalAlignmentValues::Center);
        alignment.set_wrap_text(true);
        style
            .borders_mut()
            .bottom_border_mut()
            .set_border_style(Border::BORDER_THICK);
    }
    sheet.add_merge_cells("A1:C1");
    sheet.cell_mut("A2").set_value_number(1234.5);
    sheet
        .style_mut("A2")
        .number_format_mut()
        .set_format_code("#,##0.00");
    sheet.cell_mut("B2").set_value_number(45000.5); // 2023-03-15 12:00
    sheet
        .style_mut("B2")
        .number_format_mut()
        .set_format_code("yyyy-mm-dd hh:mm");
    sheet.cell_mut("C2").set_value_bool(true);
    sheet.cell_mut("A3").set_value_number(2);
    sheet.cell_mut("B3").set_value_number(3);
    sheet.cell_mut("C3").set_formula("A3*B3").set_formula_result_default("6");
    sheet.cell_mut("A4").set_value("a\tb\n\"q\"");
    sheet.cell_mut("A5").set_value("纵向并");
    sheet.add_merge_cells("A5:A6");
    sheet.column_dimension_mut("A").set_width(20.0);
    sheet.row_dimension_mut(2).set_height(30.0);
    to_bytes(&book)
}

#[test]
fn spreadsheet_ml_has_expanded_styles_and_no_parent_or_formula() {
    let set = convert(&sample()).unwrap();
    let xml = get(&set, "XML Spreadsheet");
    assert!(xml.starts_with("<?xml"));
    assert!(!xml.contains("ss:Parent"), "{xml}");
    assert!(!xml.contains("ss:Formula"), "{xml}");
    assert!(xml.contains("<Worksheet ss:Name=\"报销\">"));
    // 合并
    assert!(xml.contains("ss:MergeAcross=\"2\""), "{xml}");
    assert!(xml.contains("ss:MergeDown=\"1\""), "{xml}");
    // 字体、填充、对齐、边框
    assert!(xml.contains("ss:FontName=\"Noto Sans\""), "{xml}");
    assert!(xml.contains("ss:Size=\"16\""), "{xml}");
    assert!(xml.contains("ss:Bold=\"1\"") && xml.contains("ss:Italic=\"1\""));
    // 未声明下划线的字体不得出现下划线（umya 的 underline() 会误报 single）
    assert_eq!(xml.matches("ss:Underline").count(), 0, "{xml}");
    assert!(xml.contains("ss:Color=\"#FF0000\""), "{xml}");
    assert!(xml.contains("<Interior ss:Color=\"#FFFF00\" ss:Pattern=\"Solid\"/>"), "{xml}");
    assert!(xml.contains("ss:Horizontal=\"Center\"") && xml.contains("ss:WrapText=\"1\""));
    assert!(xml.contains("<Border ss:Position=\"Bottom\" ss:LineStyle=\"Continuous\" ss:Weight=\"3\""), "{xml}");
    // 数字格式与值类型
    assert!(xml.contains("<NumberFormat ss:Format=\"#,##0.00\"/>"));
    assert!(xml.contains("<Data ss:Type=\"Number\">1234.5</Data>"));
    assert!(xml.contains("<Data ss:Type=\"DateTime\">2023-03-15T12:00:00.000</Data>"), "{xml}");
    assert!(xml.contains("<Data ss:Type=\"Boolean\">1</Data>"));
    // 公式写缓存值
    assert!(xml.contains("<Data ss:Type=\"Number\">6</Data>"), "{xml}");
    // 转义
    assert!(xml.contains("标题 &lt;&amp;&gt;"));
    // 列宽、行高
    assert!(xml.contains("ss:Width=\"108.75\""), "{xml}"); // 20 字符 → 145px → 108.75pt
    assert!(xml.contains("ss:Height=\"30.00\""), "{xml}");
    // 被合并覆盖的格子之后用 ss:Index 跳位
    assert!(set.notes.iter().any(|n| n.contains("公式已转为数值")));
}

#[test]
fn every_referenced_style_id_is_defined() {
    let set = convert(&sample()).unwrap();
    let xml = get(&set, "XML Spreadsheet");
    for part in xml.split("ss:StyleID=\"").skip(1) {
        let id = part.split('"').next().unwrap();
        assert!(xml.contains(&format!("<Style ss:ID=\"{id}\"")), "missing {id}");
    }
}

#[test]
fn html_has_inline_styles_spans_and_cols() {
    let set = convert(&sample()).unwrap();
    let html = get(&set, "text/html");
    assert!(html.contains("<table"));
    assert!(html.contains("colspan=\"3\""), "{html}");
    assert!(html.contains("rowspan=\"2\""), "{html}");
    assert!(html.contains("<col width=\"145\">"), "{html}");
    assert!(html.contains("font-weight:bold"));
    assert!(html.contains("background-color:#FFFF00"));
    assert!(html.contains("color:#FF0000"));
    assert!(html.contains("text-align:center"));
    assert!(html.contains("border-bottom:3px solid #000000"), "{html}");
    assert!(html.contains("white-space:pre-wrap"));
    assert!(html.contains("height:30.00pt"));
    assert!(html.contains("标题 &lt;&amp;&gt;"));
    assert!(html.contains("1,234.50"), "{html}");
    assert!(html.contains("a\tb<br>"), "{html}");
    assert!(!html.contains("<script"));
}

#[test]
fn tsv_uses_excel_quoting_tabs_and_crlf() {
    let set = convert(&sample()).unwrap();
    let tsv = get(&set, "text/plain");
    let rows: Vec<&str> = tsv.split("\r\n").collect();
    assert_eq!(rows[1], "1,234.50\t2023-03-15 12:00\tTRUE");
    assert!(tsv.contains("\"a\tb\n\"\"q\"\"\""), "{tsv:?}");
    assert!(tsv.ends_with("\r\n"));
    assert_eq!(get(&set, "UTF8_STRING"), tsv);
}

#[test]
fn formula_without_cached_value_becomes_empty_with_note() {
    let mut book = umya_spreadsheet::new_file();
    let sheet = book.sheet_mut(0).unwrap();
    sheet.cell_mut("A1").set_value_number(1);
    sheet.cell_mut("B1").set_formula("A1+1");
    let set = convert(&to_bytes(&book)).unwrap();
    assert!(set.notes.iter().any(|n| n.contains("没有缓存值")), "{:?}", set.notes);
    assert!(!get(&set, "XML Spreadsheet").contains("ss:Formula"));
}

#[test]
fn over_cell_limit_is_rejected_and_limit_itself_is_allowed() {
    let mut book = umya_spreadsheet::new_file();
    let sheet = book.sheet_mut(0).unwrap();
    sheet.cell_mut((250, 200)).set_value("x"); // 50,000 个格子
    assert!(convert(&to_bytes(&book)).is_ok());
    book.sheet_mut(0).unwrap().cell_mut((250, 201)).set_value("x"); // 50,200
    let error = convert(&to_bytes(&book)).unwrap_err();
    assert_eq!(error.code, "FORMAT_COPY_TOO_LARGE");
}

#[test]
fn corrupt_or_non_xlsx_input_is_rejected_without_panicking() {
    for bytes in [&b""[..], b"not a zip", b"PK\x03\x04garbage", &[0xD0, 0xCF, 0x11, 0xE0, 0, 0, 0, 0]] {
        assert_eq!(convert(bytes).unwrap_err().code, "FORMAT_COPY_FAILED");
    }
}

#[test]
fn date_serials_and_formats() {
    assert_eq!(serial_to_iso(1.0), "1900-01-01T00:00:00.000");
    assert_eq!(serial_to_iso(59.0), "1900-02-28T00:00:00.000");
    assert_eq!(serial_to_iso(61.0), "1900-03-01T00:00:00.000");
    assert_eq!(serial_to_iso(45000.5), "2023-03-15T12:00:00.000");
    assert_eq!(serial_to_iso(0.75), "1899-12-31T18:00:00.000");
    assert!(is_date_format("yyyy-mm-dd"));
    assert!(is_date_format("[$-804]yyyy年m月d日"));
    assert!(is_date_format("h:mm:ss"));
    assert!(!is_date_format("#,##0.00"));
    assert!(!is_date_format("0.00\"days\""));
    assert!(!is_date_format("General"));
    assert!(!is_date_format("[Red]0.0"));
}

#[test]
fn unstyled_empty_workbook_is_converted() {
    let book = umya_spreadsheet::new_file();
    let set = convert(&to_bytes(&book)).unwrap();
    assert!(get(&set, "XML Spreadsheet").contains("<Workbook"));
}

#[test]
fn qa_perf_5000_cell_workbook_p95() {
    let mut book = umya_spreadsheet::new_file();
    let sheet = book.sheet_mut(0).unwrap();
    for r in 1..=100u32 {
        for c in 1..=50u32 {
            sheet.cell_mut((c, r)).set_value_number(f64::from(r * c) + 0.25);
            if c % 5 == 0 {
                sheet.style_mut((c, r)).font_mut().set_bold(true);
            }
        }
    }
    let bytes = to_bytes(&book);
    let mut times: Vec<f64> = (0..100)
        .map(|_| {
            let start = std::time::Instant::now();
            convert(&bytes).unwrap();
            start.elapsed().as_secs_f64() * 1000.0
        })
        .collect();
    times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    eprintln!("NFR-PERF-06 xlsx 5,000 cells: p95={:.1} ms max={:.1} ms", times[94], times[99]);
    assert!(times[94] < 1000.0);
}
