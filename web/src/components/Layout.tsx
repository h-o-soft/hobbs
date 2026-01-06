import { type ParentComponent, Show } from 'solid-js';
import { A, useLocation } from '@solidjs/router';
import { useAuth } from '../stores/auth';
import { useI18n } from '../stores/i18n';

export const Layout: ParentComponent = (props) => {
  const { t, locale, setLocale } = useI18n();
  const [auth, { logout }] = useAuth();
  const location = useLocation();

  const isActive = (path: string) => location.pathname === path;

  const handleLogout = async () => {
    await logout();
  };

  const toggleLocale = () => {
    setLocale(locale() === 'ja' ? 'en' : 'ja');
  };

  return (
    <div class="min-h-screen flex flex-col">
      {/* Header */}
      <header class="border-b border-neon-cyan/20 bg-cyber-dark/50 backdrop-blur-sm">
        <div class="container mx-auto px-4">
          <div class="flex items-center justify-between h-16">
            {/* Logo */}
            <A href="/" class="font-display text-2xl font-bold text-neon-cyan text-neon-glow">
              Beryl BBS
            </A>

            {/* Navigation */}
            <Show when={auth.isAuthenticated}>
              <nav class="hidden md:flex items-center space-x-1">
                <NavLink href="/boards" active={isActive('/boards')}>{t('nav.boards')}</NavLink>
                <NavLink href="/mail" active={isActive('/mail')}>{t('nav.mail')}</NavLink>
                <NavLink href="/chat" active={isActive('/chat')}>{t('nav.chat')}</NavLink>
                <NavLink href="/files" active={isActive('/files')}>{t('nav.files')}</NavLink>
                <Show when={auth.user?.role === 'sysop' || auth.user?.role === 'subop'}>
                  <NavLink href="/admin" active={isActive('/admin')}>{t('nav.admin')}</NavLink>
                </Show>
              </nav>
            </Show>

            {/* User Info */}
            <div class="flex items-center space-x-4">
              {/* Language Toggle */}
              <button
                onClick={toggleLocale}
                class="text-xs text-gray-500 hover:text-neon-cyan transition-colors px-2 py-1 border border-gray-700 rounded"
              >
                {locale() === 'ja' ? 'EN' : 'JA'}
              </button>

              <Show
                when={auth.isAuthenticated}
                fallback={
                  <A href="/login" class="btn-primary text-sm">
                    {t('nav.login')}
                  </A>
                }
              >
                <div class="flex items-center space-x-3">
                  <Show when={auth.user && auth.user.unread_mail_count > 0}>
                    <span class="badge-pink">
                      {auth.user!.unread_mail_count}{t('nav.unreadMails')}
                    </span>
                  </Show>
                  <A
                    href="/profile"
                    class="text-sm text-gray-400 hover:text-neon-cyan transition-colors"
                  >
                    {auth.user?.nickname}
                  </A>
                  <button
                    onClick={handleLogout}
                    class="text-sm text-gray-500 hover:text-neon-pink transition-colors"
                  >
                    {t('nav.logout')}
                  </button>
                </div>
              </Show>
            </div>
          </div>

          {/* Mobile Navigation */}
          <Show when={auth.isAuthenticated}>
            <nav class="md:hidden flex items-center space-x-1 pb-3 overflow-x-auto">
              <NavLink href="/boards" active={isActive('/boards')}>{t('nav.boards')}</NavLink>
              <NavLink href="/mail" active={isActive('/mail')}>{t('nav.mail')}</NavLink>
              <NavLink href="/chat" active={isActive('/chat')}>{t('nav.chat')}</NavLink>
              <NavLink href="/files" active={isActive('/files')}>{t('nav.files')}</NavLink>
              <Show when={auth.user?.role === 'sysop' || auth.user?.role === 'subop'}>
                <NavLink href="/admin" active={isActive('/admin')}>{t('nav.admin')}</NavLink>
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
          <span class="text-neon-purple/60">Beryl BBS</span>
          {' '}- Powered by HOBBS
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
