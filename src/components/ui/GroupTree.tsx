import React, { useMemo, useState } from 'react';
import { ChevronDown, ChevronRight, FolderOpen, Folder, Inbox, Pencil, Plus, Trash2 } from 'lucide-react';
import { Group, GROUP_FILTER_PREFIX, SidebarFilter } from '../../types/snippet';

interface GroupTreeProps {
  groups: Group[];
  activeFilter: SidebarFilter;
  ungroupedCount: number;
  onSelect: (filter: SidebarFilter) => void;
  onCreate: (parentId: string | null) => void;
  onEdit: (group: Group) => void;
  onDelete: (group: Group) => void;
}

interface TreeNode {
  group: Group;
  children: TreeNode[];
}

function buildTree(groups: Group[]): TreeNode[] {
  const nodes = new Map<string, TreeNode>(groups.map(group => [group.id, { group, children: [] }]));
  const roots: TreeNode[] = [];
  for (const group of groups) {
    const node = nodes.get(group.id)!;
    const parent = group.parentId ? nodes.get(group.parentId) : undefined;
    (parent ? parent.children : roots).push(node);
  }
  return roots;
}

/** 左栏分组树：折叠、新建（+）、重命名/移动、删除、“未分组”与条目数。 */
export const GroupTree: React.FC<GroupTreeProps> = ({
  groups,
  activeFilter,
  ungroupedCount,
  onSelect,
  onCreate,
  onEdit,
  onDelete,
}) => {
  const [collapsedSection, setCollapsedSection] = useState(false);
  const [collapsed, setCollapsed] = useState<Record<string, boolean>>({});
  const roots = useMemo(() => buildTree(groups), [groups]);

  const renderNode = (node: TreeNode): React.ReactNode => {
    const { group, children } = node;
    const filter = `${GROUP_FILTER_PREFIX}${group.id}`;
    const isActive = activeFilter === filter;
    const isCollapsed = !!collapsed[group.id];
    return (
      <li key={group.id} role="treeitem" aria-expanded={children.length ? !isCollapsed : undefined} aria-selected={isActive}>
        <div className={`manager-group-row ${isActive ? 'is-active' : ''}`} style={{ paddingInlineStart: 4 + (group.depth - 1) * 14 }}>
          <button
            type="button"
            className="manager-group-caret"
            aria-label={isCollapsed ? '展开分组' : '折叠分组'}
            tabIndex={children.length ? 0 : -1}
            disabled={!children.length}
            onClick={() => setCollapsed(prev => ({ ...prev, [group.id]: !isCollapsed }))}
          >
            {children.length ? (isCollapsed
              ? <ChevronRight strokeWidth={1.5} className="size-3.5" aria-hidden />
              : <ChevronDown strokeWidth={1.5} className="size-3.5" aria-hidden />) : null}
          </button>
          <button type="button" className="manager-group-main" onClick={() => onSelect(filter)} aria-current={isActive ? 'page' : undefined}>
            {isActive || !isCollapsed && children.length
              ? <FolderOpen strokeWidth={1.5} className="size-4" aria-hidden />
              : <Folder strokeWidth={1.5} className="size-4" aria-hidden />}
            <span className="manager-side-label">{group.name}</span>
            <span className="manager-side-count">{group.snippetCount}</span>
          </button>
          <span className="manager-group-actions">
            {group.depth < 3 && (
              <button type="button" aria-label={`在 ${group.name} 下新建子分组`} title="新建子分组" onClick={() => onCreate(group.id)}>
                <Plus strokeWidth={1.5} className="size-3.5" aria-hidden />
              </button>
            )}
            <button type="button" aria-label={`重命名或移动 ${group.name}`} title="重命名或移动" onClick={() => onEdit(group)}>
              <Pencil strokeWidth={1.5} className="size-3.5" aria-hidden />
            </button>
            <button type="button" aria-label={`删除分组 ${group.name}`} title="删除分组" onClick={() => onDelete(group)}>
              <Trash2 strokeWidth={1.5} className="size-3.5" aria-hidden />
            </button>
          </span>
        </div>
        {children.length > 0 && !isCollapsed && <ul role="group" className="manager-group-list">{children.map(renderNode)}</ul>}
      </li>
    );
  };

  const ungroupedFilter = GROUP_FILTER_PREFIX;
  return (
    <div className="manager-group-tree">
      <div className="manager-side-section-row">
        <button
          type="button"
          className="manager-side-section-toggle"
          aria-expanded={!collapsedSection}
          onClick={() => setCollapsedSection(value => !value)}
        >
          {collapsedSection
            ? <ChevronRight strokeWidth={1.5} className="size-3.5" aria-hidden />
            : <ChevronDown strokeWidth={1.5} className="size-3.5" aria-hidden />}
          分组
        </button>
        <button type="button" className="manager-group-add" aria-label="新建分组" title="新建分组" onClick={() => onCreate(null)}>
          <Plus strokeWidth={1.5} className="size-3.5" aria-hidden />
        </button>
      </div>
      {!collapsedSection && (
        <ul role="tree" aria-label="分组" className="manager-group-list">
          {roots.map(renderNode)}
          <li role="treeitem" aria-selected={activeFilter === ungroupedFilter}>
            <div className={`manager-group-row ${activeFilter === ungroupedFilter ? 'is-active' : ''}`} style={{ paddingInlineStart: 4 }}>
              <span className="manager-group-caret" aria-hidden />
              <button
                type="button"
                className="manager-group-main"
                onClick={() => onSelect(ungroupedFilter)}
                aria-current={activeFilter === ungroupedFilter ? 'page' : undefined}
              >
                <Inbox strokeWidth={1.5} className="size-4" aria-hidden />
                <span className="manager-side-label">未分组</span>
                <span className="manager-side-count">{ungroupedCount}</span>
              </button>
            </div>
          </li>
        </ul>
      )}
    </div>
  );
};
