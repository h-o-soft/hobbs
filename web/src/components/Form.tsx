import type { Component, JSX } from 'solid-js';
import { Show, splitProps, For } from 'solid-js';

// Input Component
interface InputProps extends JSX.InputHTMLAttributes<HTMLInputElement> {
  label?: string;
  error?: string;
}

export const Input: Component<InputProps> = (props) => {
  const [local, inputProps] = splitProps(props, ['label', 'error', 'class']);

  return (
    <div class="space-y-1">
      <Show when={local.label}>
        <label class="block text-sm text-gray-400">{local.label}</label>
      </Show>
      <input
        {...inputProps}
        class={`input ${local.error ? 'border-neon-pink/50 focus:border-neon-pink/60' : ''} ${local.class || ''}`}
      />
      <Show when={local.error}>
        <p class="text-xs text-neon-pink">{local.error}</p>
      </Show>
    </div>
  );
};

// Textarea Component
interface TextareaProps extends JSX.TextareaHTMLAttributes<HTMLTextAreaElement> {
  label?: string;
  error?: string;
}

export const Textarea: Component<TextareaProps> = (props) => {
  const [local, textareaProps] = splitProps(props, ['label', 'error', 'class']);

  return (
    <div class="space-y-1">
      <Show when={local.label}>
        <label class="block text-sm text-gray-400">{local.label}</label>
      </Show>
      <textarea
        {...textareaProps}
        class={`input min-h-[120px] resize-y ${local.error ? 'border-neon-pink/50 focus:border-neon-pink/60' : ''} ${local.class || ''}`}
      />
      <Show when={local.error}>
        <p class="text-xs text-neon-pink">{local.error}</p>
      </Show>
    </div>
  );
};

// Select Component
interface SelectProps extends Omit<JSX.SelectHTMLAttributes<HTMLSelectElement>, 'value'> {
  label?: string;
  error?: string;
  options: Array<{ value: string; label: string }>;
  value?: string;
}

export const Select: Component<SelectProps> = (props) => {
  const [local, selectProps] = splitProps(props, ['label', 'error', 'options', 'class', 'value']);

  return (
    <div class="space-y-1">
      <Show when={local.label}>
        <label class="block text-sm text-gray-400">{local.label}</label>
      </Show>
      <select
        {...selectProps}
        value={local.value}
        class={`input ${local.error ? 'border-neon-pink/50 focus:border-neon-pink/60' : ''} ${local.class || ''}`}
      >
        <For each={local.options}>
          {(option) => (
            <option value={option.value} selected={option.value === local.value}>
              {option.label}
            </option>
          )}
        </For>
      </select>
      <Show when={local.error}>
        <p class="text-xs text-neon-pink">{local.error}</p>
      </Show>
    </div>
  );
};

// Button Component
interface ButtonProps extends JSX.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'secondary' | 'danger' | 'success';
  loading?: boolean;
}

export const Button: Component<ButtonProps> = (props) => {
  const [local, buttonProps] = splitProps(props, ['variant', 'loading', 'class', 'children', 'disabled']);

  const variantClass = {
    primary: 'btn-primary',
    secondary: 'btn-secondary',
    danger: 'btn-danger',
    success: 'btn-success',
  };

  return (
    <button
      {...buttonProps}
      disabled={local.disabled || local.loading}
      class={`${variantClass[local.variant || 'primary']} ${local.loading ? 'opacity-70 cursor-wait' : ''} ${local.class || ''}`}
    >
      <Show when={local.loading}>
        <span class="inline-block w-4 h-4 border-2 border-current border-t-transparent rounded-full animate-spin mr-2" />
      </Show>
      {local.children}
    </button>
  );
};
