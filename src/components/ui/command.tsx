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
  <div className="quick-search-input-row">
    <div className="quick-search-field">
      <Search className="size-[18px]" strokeWidth={1.5} aria-hidden />
      <CommandPrimitive.Input
        ref={ref}
        autoFocus={autoFocus}
        className={cn('quick-search-input', className)}
        {...props}
      />
    </div>
    <kbd className="raycast-kbd select-none shrink-0">Esc</kbd>
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
  <CommandPrimitive.Group className={cn('quick-search-group p-1', className)} {...props} />
);
export const CommandItem = ({ className, ...props }: React.ComponentProps<typeof CommandPrimitive.Item>) => (
  <CommandPrimitive.Item className={cn('quick-search-item', className)} {...props} />
);
export const CommandShortcut = ({ className, ...props }: React.ComponentProps<'span'>) => (
  <span className={cn('ml-auto', className)} {...props} />
);
