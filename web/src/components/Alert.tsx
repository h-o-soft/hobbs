import type { ParentComponent } from 'solid-js';
import { Show } from 'solid-js';

interface AlertProps {
  type: 'success' | 'error' | 'warning' | 'info';
  onClose?: () => void;
}

export const Alert: ParentComponent<AlertProps> = (props) => {
  const typeClasses = {
    success: 'border-neon-green/50 bg-neon-green/10 text-neon-green',
    error: 'border-neon-pink/50 bg-neon-pink/10 text-neon-pink',
    warning: 'border-neon-orange/50 bg-neon-orange/10 text-neon-orange',
    info: 'border-neon-cyan/50 bg-neon-cyan/10 text-neon-cyan',
  };

  return (
    <div class={`px-4 py-3 rounded border ${typeClasses[props.type]} relative`}>
      <div class="pr-6">
        {props.children}
      </div>
      <Show when={props.onClose}>
        <button
          onClick={props.onClose}
          class="absolute top-2 right-2 opacity-60 hover:opacity-100 transition-opacity"
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </Show>
    </div>
  );
};
