import { type Component, createSignal } from 'solid-js';
import { A, useNavigate } from '@solidjs/router';
import { useAuth } from '../stores/auth';
import { Input, Button, Alert } from '../components';
import { ApiError } from '../api/client';

export const RegisterPage: Component = () => {
  const [, { register }] = useAuth();
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
      setError('パスワードが一致しません');
      return;
    }

    if (password().length < 8) {
      setError('パスワードは8文字以上で入力してください');
      return;
    }

    setLoading(true);

    try {
      await register(username(), password(), nickname(), email() || undefined);
      navigate('/');
    } catch (err) {
      if (err instanceof ApiError) {
        setError(err.message);
      } else {
        setError('登録に失敗しました');
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
            HOBBS
          </h1>
          <p class="text-gray-500 mt-2">新規登録</p>
        </div>

        {/* Register Form */}
        <div class="card">
          <h2 class="text-xl font-medium text-neon-cyan mb-6">アカウント作成</h2>

          {error() && (
            <div class="mb-4">
              <Alert type="error" onClose={() => setError('')}>
                {error()}
              </Alert>
            </div>
          )}

          <form onSubmit={handleSubmit} class="space-y-4">
            <Input
              label="ユーザーID"
              type="text"
              value={username()}
              onInput={(e) => setUsername(e.currentTarget.value)}
              required
              maxLength={16}
              autocomplete="username"
              placeholder="英数字 1-16文字"
            />

            <Input
              label="ニックネーム"
              type="text"
              value={nickname()}
              onInput={(e) => setNickname(e.currentTarget.value)}
              required
              maxLength={20}
              placeholder="表示名 (1-20文字)"
            />

            <Input
              label="メールアドレス (任意)"
              type="email"
              value={email()}
              onInput={(e) => setEmail(e.currentTarget.value)}
              autocomplete="email"
              placeholder="example@example.com"
            />

            <Input
              label="パスワード"
              type="password"
              value={password()}
              onInput={(e) => setPassword(e.currentTarget.value)}
              required
              minLength={8}
              autocomplete="new-password"
              placeholder="8文字以上"
            />

            <Input
              label="パスワード (確認)"
              type="password"
              value={confirmPassword()}
              onInput={(e) => setConfirmPassword(e.currentTarget.value)}
              required
              autocomplete="new-password"
              placeholder="もう一度入力してください"
            />

            <Button
              type="submit"
              variant="primary"
              loading={loading()}
              class="w-full"
            >
              登録
            </Button>
          </form>

          <div class="mt-6 text-center text-sm text-gray-500">
            既にアカウントをお持ちの方は{' '}
            <A href="/login" class="text-neon-purple hover:text-neon-pink transition-colors">
              ログイン
            </A>
          </div>
        </div>
      </div>
    </div>
  );
};
