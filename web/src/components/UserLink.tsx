import { type Component } from 'solid-js';
import { A } from '@solidjs/router';

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
 *
 * @example
 * <UserLink username="john" />
 * <UserLink username="john" displayName="John Doe" />
 */
export const UserLink: Component<UserLinkProps> = (props) => {
  return (
    <A
      href={`/users/${encodeURIComponent(props.username)}`}
      class={`text-neon-cyan hover:text-neon-pink transition-colors ${props.class || ''}`}
    >
      {props.displayName || props.username}
    </A>
  );
};
