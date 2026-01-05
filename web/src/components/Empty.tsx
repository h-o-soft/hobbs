import type { Component, JSX } from 'solid-js';
import { Show } from 'solid-js';

interface EmptyProps {
  icon?: JSX.Element;
  title: string;
  description?: string;
  action?: JSX.Element;
}

export const Empty: Component<EmptyProps> = (props) => {
  return (
    <div class="flex flex-col items-center justify-center py-12 text-center">
      <Show when={props.icon}>
        <div class="text-gray-600 mb-4">
          {props.icon}
        </div>
      </Show>
      <h3 class="text-lg font-medium text-gray-400 mb-2">{props.title}</h3>
      <Show when={props.description}>
        <p class="text-sm text-gray-500 mb-4">{props.description}</p>
      </Show>
      <Show when={props.action}>
        {props.action}
      </Show>
    </div>
  );
};
