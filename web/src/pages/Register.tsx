import { type Component, createSignal } from 'solid-js';
import { A, useNavigate } from '@solidjs/router';
import { useAuth } from '../stores/auth';
import { useSiteConfig } from '../stores/siteConfig';
import { useI18n } from '../stores/i18n';
import { Input, Button, Alert } from '../components';
import { ApiError } from '../api/client';

export const RegisterPage: Component = () => {
  const { t, translateError } = useI18n();
  const [, { register }] = useAuth();
  const [siteConfig] = useSiteConfig();
  const navigate = useNavigate();

  const [username, setUsername] = createSignal('');
  const [password, setPassword] = createSignal('');
  const [confirmPassword, setConfirmPassword] = createSignal('');
  const [nickname, setNickname] = createSignal('');
  const [email, setEmail] = createSignal('');
  const [error, setError] = createSignal('');
  const [loading, setLoading] = createSignal(false);

  const handleSubmit = async (e: Event) => {
    e.preventDefault();
    setError('');

    if (password() !== confirmPassword()) {
      setError(t('auth.passwordMismatch'));
      return;
    }

    if (password().length < 8) {
      setError(t('auth.passwordTooShort'));
      return;
    }

    setLoading(true);

    try {
      await register(username(), password(), nickname(), email() || undefined);
      navigate('/');
    } catch (err) {
      if (err instanceof ApiError) {
        setError(translateError(err.code, err.message));
      } else {
        setError(t('auth.registerFailed'));
      }
    } finally {
      setLoading(false);
    }
  };

  return (
    <div class="min-h-[80vh] flex items-center justify-center py-8">
      <div class="w-full max-w-md">
        {/* Logo */}
        <div class="text-center mb-8">
          <h1 class="font-display text-4xl font-bold text-neon-cyan text-neon-glow-intense animate-pulse-neon">
            {siteConfig.config.name.split(' - ')[0] || siteConfig.config.name}
          </h1>
          <p class="text-gray-500 mt-2">{t('auth.register')}</p>
        </div>

        {/* Register Form */}
        <div class="card">
          <h2 class="text-xl font-medium text-neon-cyan mb-6">{t('auth.createAccount')}</h2>

          {/* Telnet Password Warning */}
          {siteConfig.config.telnet_enabled && (
            <div class="mb-4">
              <Alert type="warning">
                {t('auth.telnetPasswordWarning')}
              </Alert>
            </div>
          )}

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
              maxLength={16}
              autocomplete="username"
              placeholder={t('auth.usernamePlaceholder')}
            />

            <Input
              label={t('auth.nickname')}
              type="text"
              value={nickname()}
              onInput={(e) => setNickname(e.currentTarget.value)}
              required
              maxLength={20}
              placeholder={t('auth.nicknamePlaceholder')}
            />

            <Input
              label={t('auth.emailOptional')}
              type="email"
              value={email()}
              onInput={(e) => setEmail(e.currentTarget.value)}
              autocomplete="email"
              placeholder={t('auth.emailPlaceholder')}
            />

            <Input
              label={t('auth.password')}
              type="password"
              value={password()}
              onInput={(e) => setPassword(e.currentTarget.value)}
              required
              minLength={8}
              autocomplete="new-password"
              placeholder={t('auth.passwordPlaceholder')}
            />

            <Input
              label={t('auth.confirmPassword')}
              type="password"
              value={confirmPassword()}
              onInput={(e) => setConfirmPassword(e.currentTarget.value)}
              required
              autocomplete="new-password"
              placeholder={t('auth.confirmPasswordPlaceholder')}
            />

            <Button
              type="submit"
              variant="primary"
              loading={loading()}
              class="w-full"
            >
              {t('auth.register')}
            </Button>
          </form>

          <div class="mt-6 text-center text-sm text-gray-500">
            {t('auth.hasAccount')}{' '}
            <A href="/login" class="text-neon-purple hover:text-neon-pink transition-colors">
              {t('auth.loginHere')}
            </A>
          </div>
        </div>
      </div>
    </div>
  );
};
