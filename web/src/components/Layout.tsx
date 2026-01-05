import { type ParentComponent, Show } from 'solid-js';
import { A, useLocation } from '@solidjs/router';
import { useAuth } from '../stores/auth';

export const Layout: ParentComponent = (props) => {
  const [auth, { logout }] = useAuth();
  const location = useLocation();

  const isActive = (path: string) => location.pathname === path;

  const handleLogout = async () => {
    await logout();
  };

  return (
    <div class="min-h-screen flex flex-col">
      {/* Header */}
      <header class="border-b border-neon-cyan/20 bg-cyber-dark/50 backdrop-blur-sm">
        <div class="container mx-auto px-4">
          <div class="flex items-center justify-between h-16">
            {/* Logo */}
            <A href="/" class="font-display text-2xl font-bold text-neon-cyan text-neon-glow">
              HOBBS
            </A>

            {/* Navigation */}
            <Show when={auth.isAuthenticated}>
              <nav class="hidden md:flex items-center space-x-1">
                <NavLink href="/boards" active={isActive('/boards')}>掲示板</NavLink>
                <NavLink href="/mail" active={isActive('/mail')}>メール</NavLink>
                <NavLink href="/chat" active={isActive('/chat')}>チャット</NavLink>
                <NavLink href="/files" active={isActive('/files')}>ファイル</NavLink>
                <Show when={auth.user?.role === 'sysop' || auth.user?.role === 'subop'}>
                  <NavLink href="/admin" active={isActive('/admin')}>管理</NavLink>
                </Show>
              </nav>
            </Show>

            {/* User Info */}
            <div class="flex items-center space-x-4">
              <Show
                when={auth.isAuthenticated}
                fallback={
                  <A href="/login" class="btn-primary text-sm">
                    ログイン
                  </A>
                }
              >
                <div class="flex items-center space-x-3">
                  <Show when={auth.user && auth.user.unread_mail_count > 0}>
                    <span class="badge-pink">
                      {auth.user!.unread_mail_count}通
                    </span>
                  </Show>
                  <span class="text-sm text-gray-400">
                    {auth.user?.nickname}
                  </span>
                  <button
                    onClick={handleLogout}
                    class="text-sm text-gray-500 hover:text-neon-pink transition-colors"
                  >
                    ログアウト
                  </button>
                </div>
              </Show>
            </div>
          </div>

          {/* Mobile Navigation */}
          <Show when={auth.isAuthenticated}>
            <nav class="md:hidden flex items-center space-x-1 pb-3 overflow-x-auto">
              <NavLink href="/boards" active={isActive('/boards')}>掲示板</NavLink>
              <NavLink href="/mail" active={isActive('/mail')}>メール</NavLink>
              <NavLink href="/chat" active={isActive('/chat')}>チャット</NavLink>
              <NavLink href="/files" active={isActive('/files')}>ファイル</NavLink>
              <Show when={auth.user?.role === 'sysop' || auth.user?.role === 'subop'}>
                <NavLink href="/admin" active={isActive('/admin')}>管理</NavLink>
              </Show>
            </nav>
          </Show>
        </div>
      </header>

      {/* Main Content */}
      <main class="flex-1 container mx-auto px-4 py-6">
        {props.children}
      </main>

      {/* Footer */}
      <footer class="border-t border-neon-cyan/10 py-4">
        <div class="container mx-auto px-4 text-center text-xs text-gray-600">
          <span class="text-neon-purple/60">HOBBS</span>
          {' '}- Hobbyist Bulletin Board System
        </div>
      </footer>
    </div>
  );
};

interface NavLinkProps {
  href: string;
  active: boolean;
  children: any;
}

const NavLink = (props: NavLinkProps) => {
  return (
    <A
      href={props.href}
      class={`px-3 py-2 text-sm rounded transition-all duration-200 whitespace-nowrap ${
        props.active
          ? 'text-neon-cyan bg-neon-cyan/10'
          : 'text-gray-400 hover:text-neon-cyan hover:bg-neon-cyan/5'
      }`}
    >
      {props.children}
    </A>
  );
};
