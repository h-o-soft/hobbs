import { type ParentComponent, Show, createEffect, onCleanup } from 'solid-js';
import { Portal } from 'solid-js/web';

interface ModalProps {
  isOpen: boolean;
  onClose: () => void;
  title?: string;
  size?: 'sm' | 'md' | 'lg' | 'xl';
}

export const Modal: ParentComponent<ModalProps> = (props) => {
  const sizeClasses = {
    sm: 'max-w-sm',
    md: 'max-w-md',
    lg: 'max-w-lg',
    xl: 'max-w-xl',
  };

  // Handle escape key
  createEffect(() => {
    if (props.isOpen) {
      const handleKeyDown = (e: KeyboardEvent) => {
        if (e.key === 'Escape') {
          props.onClose();
        }
      };
      window.addEventListener('keydown', handleKeyDown);
      onCleanup(() => window.removeEventListener('keydown', handleKeyDown));
    }
  });

  // Prevent body scroll when modal is open
  createEffect(() => {
    if (props.isOpen) {
      document.body.style.overflow = 'hidden';
      onCleanup(() => {
        document.body.style.overflow = '';
      });
    }
  });

  return (
    <Show when={props.isOpen}>
      <Portal>
        <div class="fixed inset-0 z-50 flex items-center justify-center p-4">
          {/* Backdrop */}
          <div
            class="absolute inset-0 bg-cyber-darker/80 backdrop-blur-sm"
            onClick={props.onClose}
          />

          {/* Modal */}
          <div
            class={`relative w-full ${sizeClasses[props.size || 'md']} bg-cyber-dark border border-neon-cyan/30 rounded-lg shadow-2xl`}
            style={{
              'box-shadow': '0 0 30px rgba(0, 255, 255, 0.1), inset 0 0 30px rgba(0, 255, 255, 0.02)',
            }}
          >
            {/* Header */}
            <Show when={props.title}>
              <div class="flex items-center justify-between px-6 py-4 border-b border-neon-cyan/20">
                <h2 class="text-lg font-medium text-neon-cyan">{props.title}</h2>
                <button
                  onClick={props.onClose}
                  class="text-gray-500 hover:text-neon-pink transition-colors"
                >
                  <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                  </svg>
                </button>
              </div>
            </Show>

            {/* Content */}
            <div class="p-6">
              {props.children}
            </div>
          </div>
        </div>
      </Portal>
    </Show>
  );
};
