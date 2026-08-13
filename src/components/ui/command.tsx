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
>(({ className, ...props }, ref) => (
  <div className="flex h-14 items-center gap-3 border-b border-[color:var(--quick-border)] px-4">
    <Search className="size-5 shrink-0 text-[color:var(--quick-muted)]" aria-hidden />
    <CommandPrimitive.Input
      ref={ref}
      className={cn('h-full w-full bg-transparent text-base text-[color:var(--quick-fg)] outline-none placeholder:text-[color:var(--quick-muted)]', className)}
      {...props}
    />
  </div>
));
CommandInput.displayName = 'CommandInput';

export const CommandList = ({ className, ...props }: React.ComponentProps<typeof CommandPrimitive.List>) => (
  <CommandPrimitive.List className={cn('overflow-x-hidden overflow-y-auto', className)} {...props} />
);
export const CommandEmpty = ({ className, ...props }: React.ComponentProps<typeof CommandPrimitive.Empty>) => (
  <CommandPrimitive.Empty className={cn('py-12 text-center text-sm', className)} {...props} />
);
export const CommandGroup = ({ className, ...props }: React.ComponentProps<typeof CommandPrimitive.Group>) => (
  <CommandPrimitive.Group className={cn('p-2 [&_[cmdk-group-heading]]:px-2 [&_[cmdk-group-heading]]:py-1.5 [&_[cmdk-group-heading]]:text-[11px] [&_[cmdk-group-heading]]:font-semibold [&_[cmdk-group-heading]]:uppercase [&_[cmdk-group-heading]]:tracking-wider [&_[cmdk-group-heading]]:text-[color:var(--quick-muted)]', className)} {...props} />
);
export const CommandItem = ({ className, ...props }: React.ComponentProps<typeof CommandPrimitive.Item>) => (
  <CommandPrimitive.Item className={cn('flex cursor-default items-center gap-2 rounded-lg px-2 py-2 text-sm outline-none data-[selected=true]:bg-[color:var(--quick-active)]', className)} {...props} />
);
export const CommandShortcut = ({ className, ...props }: React.ComponentProps<'span'>) => (
  <span className={cn('ml-auto font-mono text-[11px] text-[color:var(--quick-muted)]', className)} {...props} />
);
