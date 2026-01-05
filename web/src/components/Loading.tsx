import type { Component } from 'solid-js';

interface LoadingProps {
  size?: 'sm' | 'md' | 'lg';
  text?: string;
}

export const Loading: Component<LoadingProps> = (props) => {
  const sizeClasses = {
    sm: 'w-4 h-4',
    md: 'w-8 h-8',
    lg: 'w-12 h-12',
  };

  return (
    <div class="flex flex-col items-center justify-center space-y-3">
      <div
        class={`${sizeClasses[props.size || 'md']} border-2 border-neon-cyan/30 border-t-neon-cyan rounded-full animate-spin`}
      />
      {props.text && (
        <span class="text-sm text-gray-400">{props.text}</span>
      )}
    </div>
  );
};

export const PageLoading: Component = () => {
  return (
    <div class="min-h-[50vh] flex items-center justify-center">
      <Loading size="lg" text="読み込み中..." />
    </div>
  );
};
