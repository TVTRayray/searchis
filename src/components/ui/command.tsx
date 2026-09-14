import * as React from 'react';
import { Command as CommandPrimitive } from 'cmdk';
import { Search } from 'lucide-react';
import { cn } from '../../lib/utils';

export const Command = ({ className, ...props }: React.ComponentProps<typeof CommandPrimitive>) => (
  <CommandPrimitive className={cn('flex h-full w-full flex-col overflow-hidden', className)} {...props} />
);

export const CommandInput = React.forwardRef<
  React.ElementRef<typeof CommandPrimitive.Input>,
  React.ComponentPropsWithoutRef<typeof CommandPrimitive.Input>
>(({ className, autoFocus = true, ...props }, ref) => (
  <div className="relative z-10 flex h-[52px] items-center gap-3 border-b border-[color:var(--color-border)] px-4">
    <Search className="size-4 shrink-0 text-[color:var(--color-fg-muted)]" aria-hidden />
    <CommandPrimitive.Input
      ref={ref}
      autoFocus={autoFocus}
      className={cn('h-full w-full bg-transparent text-[14.5px] font-normal text-[color:var(--color-fg)] outline-none placeholder:text-[color:var(--color-fg-muted)]', className)}
      {...props}
    />
    <kbd className="raycast-kbd select-none text-[10px] shrink-0">Esc</kbd>
  </div>
));
CommandInput.displayName = 'CommandInput';

export const CommandList = ({ className, ...props }: React.ComponentProps<typeof CommandPrimitive.List>) => (
  <CommandPrimitive.List className={cn('relative z-10 overflow-x-hidden overflow-y-auto p-1.5', className)} {...props} />
);
export const CommandEmpty = ({ className, ...props }: React.ComponentProps<typeof CommandPrimitive.Empty>) => (
  <CommandPrimitive.Empty className={cn('py-12 text-center text-sm', className)} {...props} />
);
export const CommandGroup = ({ className, ...props }: React.ComponentProps<typeof CommandPrimitive.Group>) => (
  <CommandPrimitive.Group className={cn('p-1 [&_[cmdk-group-heading]]:px-3 [&_[cmdk-group-heading]]:py-2 [&_[cmdk-group-heading]]:text-[11px] [&_[cmdk-group-heading]]:font-semibold [&_[cmdk-group-heading]]:tracking-wide [&_[cmdk-group-heading]]:text-[color:var(--color-fg-muted)]', className)} {...props} />
);
export const CommandItem = ({ className, ...props }: React.ComponentProps<typeof CommandPrimitive.Item>) => (
  <CommandPrimitive.Item className={cn('relative z-10 flex cursor-pointer items-center gap-3 rounded-[12px] px-2.5 py-2 mx-1 my-0.5 text-sm outline-none transition-all duration-120 data-[selected=true]:bg-[color:var(--sel-bg,rgba(255,255,255,0.15))] data-[selected=true]:shadow-[inset_0_0_0_1px_rgba(255,255,255,0.15),0_2px_8px_rgba(0,0,0,0.20)]', className)} {...props} />
);
export const CommandShortcut = ({ className, ...props }: React.ComponentProps<'span'>) => (
  <span className={cn('ml-auto font-mono text-[11px] text-[color:var(--color-fg-muted)]', className)} {...props} />
);
