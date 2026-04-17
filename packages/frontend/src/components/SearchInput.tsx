import type { Component } from 'solid-js';
import { cn } from '@/lib/cn';

export const SearchInput: Component<{
  value: string;
  onInput: (v: string) => void;
  onSubmit: () => void;
  placeholder?: string;
  autofocus?: boolean;
  class?: string;
}> = (props) => {
  return (
    <div
      class={cn(
        'flex items-center rounded border border-gray-300 bg-white focus-within:border-gray-900',
        props.class,
      )}
    >
      <input
        type="text"
        value={props.value}
        onInput={(e) => props.onInput(e.currentTarget.value)}
        onKeyDown={(e) => e.key === 'Enter' && props.onSubmit()}
        placeholder={props.placeholder ?? 'Search repos, code, docs…'}
        autofocus={props.autofocus}
        class="flex-1 bg-transparent px-4 py-2.5 font-mono text-sm outline-none placeholder:text-gray-400"
      />
    </div>
  );
};
