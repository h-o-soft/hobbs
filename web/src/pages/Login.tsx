import { type Component, createSignal } from 'solid-js';
import { A, useNavigate } from '@solidjs/router';
import { useAuth } from '../stores/auth';
import { useI18n } from '../stores/i18n';
import { Input, Button, Alert } from '../components';
import { ApiError } from '../api/client';

export const LoginPage: Component = () => {
  const { t, translateError } = useI18n();
  const [, { login }] = useAuth();
  const navigate = useNavigate();

  const [username, setUsername] = createSignal('');
  const [password, setPassword] = createSignal('');
  const [error, setError] = createSignal('');
  const [loading, setLoading] = createSignal(false);

  const handleSubmit = async (e: Event) => {
    e.preventDefault();
    setError('');
    setLoading(true);

    try {
      await login(username(), password());
      navigate('/');
    } catch (err) {
      if (err instanceof ApiError) {
        setError(translateError(err.code, err.message));
      } else {
        setError(t('auth.loginFailed'));
      }
    } finally {
      setLoading(false);
    }
  };

  return (
    <div class="min-h-[80vh] flex items-center justify-center">
      <div class="w-full max-w-md">
        {/* Logo */}
        <div class="text-center mb-8">
          <h1 class="font-display text-4xl font-bold text-neon-cyan text-neon-glow-intense animate-pulse-neon">
            Beryl BBS
          </h1>
          <p class="text-gray-500 mt-2">{t('auth.subtitle')}</p>
        </div>

        {/* Login Form */}
        <div class="card">
          <h2 class="text-xl font-medium text-neon-cyan mb-6">{t('auth.login')}</h2>

          {error() && (
            <div class="mb-4">
              <Alert type="error" onClose={() => setError('')}>
                {error()}
              </Alert>
            </div>
          )}

          <form onSubmit={handleSubmit} class="space-y-4">
            <Input
              label={t('auth.username')}
              type="text"
              value={username()}
              onInput={(e) => setUsername(e.currentTarget.value)}
              required
              autocomplete="username"
            />

            <Input
              label={t('auth.password')}
              type="password"
              value={password()}
              onInput={(e) => setPassword(e.currentTarget.value)}
              required
              autocomplete="current-password"
            />

            <Button
              type="submit"
              variant="primary"
              loading={loading()}
              class="w-full"
            >
              {t('auth.login')}
            </Button>
          </form>

          <div class="mt-6 text-center text-sm text-gray-500">
            {t('auth.noAccount')}{' '}
            <A href="/register" class="text-neon-purple hover:text-neon-pink transition-colors">
              {t('auth.registerHere')}
            </A>
          </div>
        </div>

        {/* Decorative Element */}
        <div class="mt-8 text-center">
          <div class="inline-block px-4 py-1 border border-neon-purple/30 rounded text-xs text-gray-600">
            <span class="text-neon-purple/60">SYSTEM</span> ONLINE
          </div>
        </div>
      </div>
    </div>
  );
};
