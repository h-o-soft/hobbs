import type { Component } from 'solid-js';
import { For, Show } from 'solid-js';

interface PaginationProps {
  page: number;
  totalPages: number;
  onPageChange: (page: number) => void;
}

export const Pagination: Component<PaginationProps> = (props) => {
  const getPageNumbers = () => {
    const pages: (number | string)[] = [];
    const current = props.page;
    const total = props.totalPages;

    if (total <= 7) {
      for (let i = 1; i <= total; i++) {
        pages.push(i);
      }
    } else {
      pages.push(1);

      if (current > 3) {
        pages.push('...');
      }

      const start = Math.max(2, current - 1);
      const end = Math.min(total - 1, current + 1);

      for (let i = start; i <= end; i++) {
        pages.push(i);
      }

      if (current < total - 2) {
        pages.push('...');
      }

      pages.push(total);
    }

    return pages;
  };

  return (
    <Show when={props.totalPages >= 1}>
      <div class="flex items-center justify-center space-x-1">
        {/* Previous */}
        <button
          onClick={() => props.onPageChange(props.page - 1)}
          disabled={props.page <= 1}
          class="px-3 py-1 text-sm rounded border border-neon-cyan/30 text-gray-400
                 hover:text-neon-cyan hover:border-neon-cyan/50
                 disabled:opacity-30 disabled:cursor-not-allowed
                 transition-all duration-200"
        >
          &lt;
        </button>

        {/* Page Numbers */}
        <For each={getPageNumbers()}>
          {(page) => (
            <Show
              when={typeof page === 'number'}
              fallback={
                <span class="px-2 text-gray-500">...</span>
              }
            >
              <button
                onClick={() => props.onPageChange(page as number)}
                class={`px-3 py-1 text-sm rounded border transition-all duration-200 ${
                  page === props.page
                    ? 'bg-neon-cyan/20 border-neon-cyan/50 text-neon-cyan'
                    : 'border-neon-cyan/30 text-gray-400 hover:text-neon-cyan hover:border-neon-cyan/50'
                }`}
              >
                {page}
              </button>
            </Show>
          )}
        </For>

        {/* Next */}
        <button
          onClick={() => props.onPageChange(props.page + 1)}
          disabled={props.page >= props.totalPages}
          class="px-3 py-1 text-sm rounded border border-neon-cyan/30 text-gray-400
                 hover:text-neon-cyan hover:border-neon-cyan/50
                 disabled:opacity-30 disabled:cursor-not-allowed
                 transition-all duration-200"
        >
          &gt;
        </button>
      </div>
    </Show>
  );
};
