import { type Component, createMemo } from 'solid-js';
import { ansiToHtml } from '../utils/ansi';

interface AnsiTextProps {
  text: string;
  class?: string;
}

/**
 * Component to render text with ANSI escape sequences as HTML.
 *
 * Converts ^[ notation and ANSI codes to styled HTML spans.
 * Preserves whitespace and newlines.
 */
export const AnsiText: Component<AnsiTextProps> = (props) => {
  const html = createMemo(() => ansiToHtml(props.text));

  return (
    <div
      class={props.class}
      style={{ "white-space": "pre-wrap" }}
      innerHTML={html()}
    />
  );
};
