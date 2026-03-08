import { type Component, Show } from 'solid-js';
import { A } from '@solidjs/router';
import { useAuth } from '../stores/auth';

export interface UserLinkProps {
  /** Username to link to */
  username: string;
  /** Display text (defaults to username) */
  displayName?: string;
  /** Additional CSS classes */
  class?: string;
}

/**
 * UserLink component - Creates a link to a user's profile page.
 * When the user is not authenticated, displays as plain text instead.
 *
 * @example
 * <UserLink username="john" />
 * <UserLink username="john" displayName="John Doe" />
 */
export const UserLink: Component<UserLinkProps> = (props) => {
  const [auth] = useAuth();

  return (
    <Show when={auth.isAuthenticated} fallback={
      <span class={`text-neon-cyan ${props.class || ''}`}>
        {props.displayName || props.username}
      </span>
    }>
      <A
        href={`/users/${encodeURIComponent(props.username)}`}
        class={`text-neon-cyan hover:text-neon-pink transition-colors ${props.class || ''}`}
      >
        {props.displayName || props.username}
      </A>
    </Show>
  );
};
