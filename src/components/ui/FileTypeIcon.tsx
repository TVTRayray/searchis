import React from 'react';
import { FileCode, FileSpreadsheet, FileText, FileType } from 'lucide-react';

export type FileCategory = 'doc' | 'sheet' | 'pdf' | 'code';

const SHEET = new Set(['xlsx', 'xls', 'xlsm', 'csv', 'tsv', 'ods']);
const DOC = new Set(['md', 'markdown', 'txt', 'doc', 'docx', 'odt', 'rtf', 'ppt', 'pptx', 'odp']);

export const fileCategory = (ext: string | null | undefined): FileCategory => {
  const value = (ext ?? '').toLowerCase();
  if (SHEET.has(value)) return 'sheet';
  if (value === 'pdf') return 'pdf';
  if (DOC.has(value)) return 'doc';
  return 'code';
};

/** 文件模板的类型图标，颜色来自文件类型 token。 */
export const FileTypeIcon: React.FC<{ ext: string | null | undefined; className?: string }> = ({ ext, className = 'size-3.5' }) => {
  const category = fileCategory(ext);
  const props = { strokeWidth: 1.5, className, 'aria-hidden': true } as const;
  switch (category) {
    case 'sheet': return <FileSpreadsheet {...props} />;
    case 'pdf': return <FileType {...props} />;
    case 'doc': return <FileText {...props} />;
    default: return <FileCode {...props} />;
  }
};

export const formatBytes = (value: number | null | undefined): string => {
  if (value === null || value === undefined) return '—';
  if (value < 1024) return `${value} B`;
  if (value < 1024 * 1024) return `${(value / 1024).toFixed(1)} KB`;
  return `${(value / 1024 / 1024).toFixed(1)} MB`;
};

/** 文件名 = 标题 + 扩展名（标题与文件名双向联动）。 */
export const templateFileName = (title: string, ext: string | null | undefined): string =>
  ext ? `${title}.${ext}` : title;
