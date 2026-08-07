import { Snippet } from '../types/snippet';

export const initialSnippets: Snippet[] = [
  {
    id: 'snip-1',
    key: 'addr-company',
    title: '公司总部地址',
    content: '北京市朝阳区科技园区创新大厦 88 号 A 座 1608 室 (邮编: 100102)',
    aliases: ['addr', '地址', '公司地址', 'office'],
    tags: ['公司', '常用'],
    pinned: true,
    usageCount: 42,
    lastUsedAt: '2026-08-01T08:30:00.000Z',
    createdAt: '2026-01-10T10:00:00.000Z',
    updatedAt: '2026-07-25T14:20:00.000Z'
  },
  {
    id: 'snip-2',
    key: 'email-work',
    title: '工作官方邮箱',
    content: 'alex.chen@innovate-tech.io',
    aliases: ['mail', '邮箱', 'email', 'workmail'],
    tags: ['常用', '联系方式'],
    pinned: true,
    usageCount: 89,
    lastUsedAt: '2026-08-01T08:45:00.000Z',
    createdAt: '2026-01-05T09:15:00.000Z',
    updatedAt: '2026-07-20T11:00:00.000Z'
  },
  {
    id: 'snip-3',
    key: 'reply-delay',
    title: '稍后回复致歉模板',
    content: '您好！收到您的邮件。我现在正在参加紧急技术会议，预计在今天下午 16:30 前给您详细答复，请谅解！',
    aliases: ['busy', '忙碌', '回复', 'delay'],
    tags: ['回复', '模板'],
    pinned: false,
    usageCount: 18,
    lastUsedAt: '2026-07-30T15:10:00.000Z',
    createdAt: '2026-02-14T16:00:00.000Z',
    updatedAt: '2026-07-28T09:00:00.000Z'
  },
  {
    id: 'snip-4',
    key: 'api-auth-header',
    title: 'API Bearer Authorization Header',
    content: 'Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkFsZXgiLCJpYXQiOjE1MTYyMzkwMjJ9.signature_sample',
    aliases: ['bearer', 'jwt', 'auth', 'header'],
    tags: ['开发', 'API'],
    pinned: true,
    sensitive: true,
    usageCount: 35,
    lastUsedAt: '2026-07-31T17:20:00.000Z',
    createdAt: '2026-03-01T11:30:00.000Z',
    updatedAt: '2026-07-30T10:15:00.000Z'
  },
  {
    id: 'snip-5',
    key: 'sql-page-query',
    title: 'PostgreSQL 高效分页模板',
    content: `SELECT id, name, created_at, status
FROM user_orders
WHERE tenant_id = $1 AND is_deleted = FALSE
ORDER BY created_at DESC
OFFSET $2 LIMIT $3;`,
    aliases: ['sql', 'page', 'pg', 'offset'],
    tags: ['开发', '数据库'],
    pinned: false,
    usageCount: 27,
    lastUsedAt: '2026-07-29T14:00:00.000Z',
    createdAt: '2026-03-12T08:20:00.000Z',
    updatedAt: '2026-07-25T16:00:00.000Z'
  },
  {
    id: 'snip-6',
    key: 'react-use-debounce',
    title: 'React Custom Hook: useDebounce',
    content: `import { useState, useEffect } from 'react';

export function useDebounce<T>(value: T, delay: number = 300): T {
  const [debouncedValue, setDebouncedValue] = useState<T>(value);

  useEffect(() => {
    const handler = setTimeout(() => setDebouncedValue(value), delay);
    return () => clearTimeout(handler);
  }, [value, delay]);

  return debouncedValue;
}`,
    aliases: ['hook', 'debounce', 'react', 'ts'],
    tags: ['开发', '前端'],
    pinned: false,
    usageCount: 14,
    lastUsedAt: '2026-07-27T10:00:00.000Z',
    createdAt: '2026-04-05T13:40:00.000Z',
    updatedAt: '2026-07-20T08:30:00.000Z'
  },
  {
    id: 'snip-7',
    key: 'git-commit-template',
    title: 'Standard Conventional Commit Message',
    content: 'feat(scope): 描述新功能的核心逻辑\n\n- 详细说明变更点 1\n- 详细说明变更点 2\n\nCloses #102',
    aliases: ['git', 'commit', 'conventional'],
    tags: ['开发', 'Git'],
    pinned: false,
    usageCount: 56,
    lastUsedAt: '2026-08-01T07:15:00.000Z',
    createdAt: '2026-02-01T15:00:00.000Z',
    updatedAt: '2026-07-15T12:00:00.000Z'
  },
  {
    id: 'snip-8',
    key: 'docker-compose-redis',
    title: 'Docker Compose Redis Stack',
    content: `version: '3.8'
services:
  redis:
    image: redis:7-alpine
    container_name: local-redis
    ports:
      - "6379:6379"
    command: redis-server --save 60 1 --loglevel notice
    volumes:
      - redis_data:/data
volumes:
  redis_data:`,
    aliases: ['docker', 'redis', 'compose', 'cache'],
    tags: ['开发', 'DevOps'],
    pinned: false,
    usageCount: 9,
    lastUsedAt: '2026-07-18T16:00:00.000Z',
    createdAt: '2026-05-10T09:00:00.000Z',
    updatedAt: '2026-07-10T14:30:00.000Z'
  },
  {
    id: 'snip-9',
    key: 'invoice-tax-info',
    title: '增值税普通发票开票抬头与税号',
    content: `单位名称：北京创新未来科技有限公司
统一社会信用代码：91110108MA01XXXXXX
开户银行：中国工商银行北京海淀支行
银行账号：0200 0045 1920 0188 888
公司电话：010-88886666`,
    aliases: ['tax', 'fapiao', '发票', '税号', '抬头'],
    tags: ['公司', '财务'],
    pinned: false,
    usageCount: 23,
    lastUsedAt: '2026-07-28T11:40:00.000Z',
    createdAt: '2026-01-20T14:00:00.000Z',
    updatedAt: '2026-06-12T10:00:00.000Z'
  }
];
