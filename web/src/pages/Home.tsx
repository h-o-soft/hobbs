import { type Component, createResource, For, Show } from 'solid-js';
import { A } from '@solidjs/router';
import { useAuth } from '../stores/auth';
import { useI18n } from '../stores/i18n';
import { PageLoading } from '../components';
import * as boardApi from '../api/board';
import type { Board } from '../types';

export const HomePage: Component = () => {
  const { t } = useI18n();
  const [auth] = useAuth();

  const [boards] = createResource(async () => {
    try {
      return await boardApi.getBoards();
    } catch {
      return [];
    }
  });

  return (
    <div class="space-y-8">
      {/* Welcome Banner */}
      <div class="card border-neon-purple/30 relative overflow-hidden">
        <div class="absolute inset-0 bg-gradient-to-r from-neon-purple/5 to-neon-cyan/5" />
        <div class="relative">
          <h1 class="font-display text-3xl font-bold text-neon-cyan mb-2">
            {t('home.welcome')}
          </h1>
          <p class="text-gray-400">
            {t('home.welcomeUser', { name: auth.user?.nickname || '' })}
          </p>
          <Show when={auth.user && auth.user.unread_mail_count > 0}>
            <div class="mt-4">
              <A href="/mail" class="inline-flex items-center space-x-2 text-neon-pink hover:text-neon-pink/80 transition-colors">
                <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 8l7.89 5.26a2 2 0 002.22 0L21 8M5 19h14a2 2 0 002-2V7a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" />
                </svg>
                <span>{auth.user!.unread_mail_count}{t('home.unreadMails')}</span>
              </A>
            </div>
          </Show>
        </div>
      </div>

      {/* Quick Access */}
      <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
        <QuickAccessCard
          href="/boards"
          title={t('nav.boards')}
          icon={
            <svg class="w-8 h-8" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
            </svg>
          }
          color="cyan"
        />
        <QuickAccessCard
          href="/mail"
          title={t('nav.mail')}
          icon={
            <svg class="w-8 h-8" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 8l7.89 5.26a2 2 0 002.22 0L21 8M5 19h14a2 2 0 002-2V7a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" />
            </svg>
          }
          color="pink"
          badge={auth.user?.unread_mail_count || 0}
        />
        <QuickAccessCard
          href="/chat"
          title={t('nav.chat')}
          icon={
            <svg class="w-8 h-8" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z" />
            </svg>
          }
          color="green"
        />
        <QuickAccessCard
          href="/files"
          title={t('nav.files')}
          icon={
            <svg class="w-8 h-8" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
            </svg>
          }
          color="purple"
        />
      </div>

      {/* Boards List */}
      <div>
        <h2 class="text-xl font-medium text-neon-cyan mb-4">{t('home.boardList')}</h2>
        <Show when={!boards.loading} fallback={<PageLoading />}>
          <div class="space-y-2">
            <For each={boards()} fallback={
              <div class="card text-center text-gray-500">
                {t('boards.noBoards')}
              </div>
            }>
              {(board) => <BoardCard board={board} />}
            </For>
          </div>
        </Show>
      </div>
    </div>
  );
};

interface QuickAccessCardProps {
  href: string;
  title: string;
  icon: any;
  color: 'cyan' | 'pink' | 'green' | 'purple';
  badge?: number;
}

const QuickAccessCard: Component<QuickAccessCardProps> = (props) => {
  const colorClasses = {
    cyan: 'hover:border-neon-cyan/50 hover:shadow-neon-cyan/20 text-neon-cyan',
    pink: 'hover:border-neon-pink/50 hover:shadow-neon-pink/20 text-neon-pink',
    green: 'hover:border-neon-green/50 hover:shadow-neon-green/20 text-neon-green',
    purple: 'hover:border-neon-purple/50 hover:shadow-neon-purple/20 text-neon-purple',
  };

  return (
    <A
      href={props.href}
      class={`card-hover flex flex-col items-center justify-center py-6 relative ${colorClasses[props.color]}`}
    >
      <Show when={props.badge && props.badge > 0}>
        <span class="absolute top-2 right-2 px-2 py-0.5 text-xs bg-neon-pink/20 text-neon-pink rounded">
          {props.badge}
        </span>
      </Show>
      <div class="mb-2">{props.icon}</div>
      <span class="text-sm text-gray-400">{props.title}</span>
    </A>
  );
};

interface BoardCardProps {
  board: Board;
}

const BoardCard: Component<BoardCardProps> = (props) => {
  const { t } = useI18n();

  return (
    <A
      href={`/boards/${props.board.id}`}
      class="card-hover block"
    >
      <div class="flex items-center justify-between">
        <div>
          <h3 class="font-medium text-gray-200">{props.board.name}</h3>
          <Show when={props.board.description}>
            <p class="text-sm text-gray-500 mt-1">{props.board.description}</p>
          </Show>
        </div>
        <div class="flex items-center space-x-4 text-xs text-gray-500">
          <span class="badge-cyan">
            {props.board.board_type === 'thread' ? t('boards.threadType') : t('boards.flatType')}
          </span>
          <span>{props.board.post_count ?? 0} {t('home.posts')}</span>
        </div>
      </div>
    </A>
  );
};
