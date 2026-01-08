import { type Component, createResource, createSignal, For, Show, onMount } from 'solid-js';
import { useSearchParams } from '@solidjs/router';
import { PageLoading, Pagination, Button, Input, Textarea, Modal, Alert, Empty, UserLink, AnsiText } from '../components';
import * as mailApi from '../api/mail';
import type { MailListItem, Mail } from '../types';
import { useI18n } from '../stores/i18n';

export const MailPage: Component = () => {
  const { t } = useI18n();
  const [searchParams, setSearchParams] = useSearchParams();
  const [activeTab, setActiveTab] = createSignal<'inbox' | 'sent'>('inbox');
  const [page, setPage] = createSignal(1);
  const [selectedMail, setSelectedMail] = createSignal<Mail | null>(null);
  const [showCompose, setShowCompose] = createSignal(false);
  const [replyTo, setReplyTo] = createSignal<Mail | null>(null);
  const [defaultRecipient, setDefaultRecipient] = createSignal<string>('');

  // Handle ?to=username query parameter
  onMount(() => {
    const toParam = searchParams.to;
    if (toParam) {
      // Handle both string and string[] cases
      const recipient = Array.isArray(toParam) ? toParam[0] : toParam;
      setDefaultRecipient(recipient);
      setShowCompose(true);
      // Clear the query parameter
      setSearchParams({ to: undefined });
    }
  });

  const [inbox, { refetch: refetchInbox }] = createResource(
    () => ({ tab: activeTab(), page: page() }),
    async ({ tab, page }) => {
      if (tab !== 'inbox') return null;
      return mailApi.getInbox({ page, per_page: 20 });
    }
  );

  const [sent, { refetch: refetchSent }] = createResource(
    () => ({ tab: activeTab(), page: page() }),
    async ({ tab, page }) => {
      if (tab !== 'sent') return null;
      return mailApi.getSent({ page, per_page: 20 });
    }
  );

  const currentList = () => activeTab() === 'inbox' ? inbox() : sent();
  const isLoading = () => activeTab() === 'inbox' ? inbox.loading : sent.loading;

  const handleTabChange = (tab: 'inbox' | 'sent') => {
    setActiveTab(tab);
    setPage(1);
  };

  const handleMailClick = async (mailItem: MailListItem) => {
    const mail = await mailApi.getMail(mailItem.id);
    setSelectedMail(mail);
    if (!mailItem.is_read && activeTab() === 'inbox') {
      refetchInbox();
    }
  };

  const handleReply = (mail: Mail) => {
    setReplyTo(mail);
    setSelectedMail(null);
    setShowCompose(true);
  };

  const handleDelete = async (id: number) => {
    if (!confirm(t('mail.confirmDelete'))) return;
    await mailApi.deleteMail(id);
    setSelectedMail(null);
    if (activeTab() === 'inbox') {
      refetchInbox();
    } else {
      refetchSent();
    }
  };

  const handleComposed = () => {
    setShowCompose(false);
    setReplyTo(null);
    setDefaultRecipient('');
    refetchSent();
  };

  return (
    <div class="space-y-6">
      {/* Header */}
      <div class="flex items-center justify-between">
        <h1 class="text-2xl font-display font-bold text-neon-cyan">{t('mail.title')}</h1>
        <Button variant="primary" onClick={() => setShowCompose(true)}>
          {t('mail.compose')}
        </Button>
      </div>

      {/* Tabs */}
      <div class="flex space-x-1 border-b border-neon-cyan/20">
        <TabButton
          active={activeTab() === 'inbox'}
          onClick={() => handleTabChange('inbox')}
        >
          {t('mail.inbox')}
        </TabButton>
        <TabButton
          active={activeTab() === 'sent'}
          onClick={() => handleTabChange('sent')}
        >
          {t('mail.sent')}
        </TabButton>
      </div>

      {/* Mail List */}
      <Show when={!isLoading()} fallback={<PageLoading />}>
        <Show
          when={currentList()?.data && currentList()!.data.length > 0}
          fallback={
            <Empty
              title={activeTab() === 'inbox' ? t('mail.noInbox') : t('mail.noSent')}
            />
          }
        >
          <div class="space-y-2">
            <For each={currentList()!.data}>
              {(mail) => (
                <div
                  onClick={() => handleMailClick(mail)}
                  class={`card-hover cursor-pointer ${!mail.is_read && activeTab() === 'inbox' ? 'border-neon-pink/30' : ''}`}
                >
                  <div class="flex items-center justify-between">
                    <div class="flex-1 min-w-0">
                      <div class="flex items-center space-x-2">
                        <Show when={!mail.is_read && activeTab() === 'inbox'}>
                          <span class="w-2 h-2 bg-neon-pink rounded-full" />
                        </Show>
                        <span class="font-medium text-gray-200 truncate">
                          {mail.subject}
                        </span>
                      </div>
                      <p class="text-sm text-gray-500 mt-1">
                        {activeTab() === 'inbox'
                          ? `${t('mail.from')}: ${mail.sender.nickname}`
                          : `${t('mail.to')}: ${mail.recipient.nickname}`}
                      </p>
                    </div>
                    <span class="text-xs text-gray-500 ml-4">
                      {formatDate(mail.created_at)}
                    </span>
                  </div>
                </div>
              )}
            </For>
          </div>

          <Pagination
            page={currentList()!.meta.page}
            totalPages={Math.ceil(currentList()!.meta.total / currentList()!.meta.per_page)}
            onPageChange={setPage}
          />
        </Show>
      </Show>

      {/* Mail Detail Modal */}
      <Modal
        isOpen={selectedMail() !== null}
        onClose={() => setSelectedMail(null)}
        title={t('mail.title')}
        size="lg"
      >
        <Show when={selectedMail()}>
          {(mail) => (
            <div class="space-y-4">
              <div class="border-b border-neon-cyan/20 pb-4">
                <h3 class="text-lg font-medium text-gray-200">{mail().subject}</h3>
                <div class="text-sm text-gray-500 mt-2 space-y-1">
                  <p>
                    {t('mail.from')}:{' '}
                    <UserLink
                      username={mail().sender.username}
                      displayName={mail().sender.nickname}
                    />
                  </p>
                  <p>
                    {t('mail.to')}:{' '}
                    <UserLink
                      username={mail().recipient.username}
                      displayName={mail().recipient.nickname}
                    />
                  </p>
                  <p>{t('mail.date')}: {formatDate(mail().created_at)}</p>
                </div>
              </div>
              <AnsiText text={mail().body} class="text-gray-300" />
              <div class="flex justify-end space-x-3 pt-4">
                <Button variant="danger" onClick={() => handleDelete(mail().id)}>
                  {t('mail.delete')}
                </Button>
                <Show when={activeTab() === 'inbox'}>
                  <Button variant="primary" onClick={() => handleReply(mail())}>
                    {t('mail.reply')}
                  </Button>
                </Show>
              </div>
            </div>
          )}
        </Show>
      </Modal>

      {/* Compose Modal */}
      <Modal
        isOpen={showCompose()}
        onClose={() => { setShowCompose(false); setReplyTo(null); setDefaultRecipient(''); }}
        title={replyTo() ? t('mail.reply') : t('mail.newMail')}
        size="lg"
      >
        <ComposeForm
          replyTo={replyTo()}
          defaultRecipient={defaultRecipient()}
          onSuccess={handleComposed}
          onCancel={() => { setShowCompose(false); setReplyTo(null); setDefaultRecipient(''); }}
        />
      </Modal>
    </div>
  );
};

interface TabButtonProps {
  active: boolean;
  onClick: () => void;
  children: any;
}

const TabButton: Component<TabButtonProps> = (props) => {
  return (
    <button
      onClick={props.onClick}
      class={`px-4 py-2 text-sm font-medium transition-all duration-200 border-b-2 -mb-px ${
        props.active
          ? 'text-neon-cyan border-neon-cyan'
          : 'text-gray-500 border-transparent hover:text-gray-300'
      }`}
    >
      {props.children}
    </button>
  );
};

interface ComposeFormProps {
  replyTo: Mail | null;
  defaultRecipient?: string;
  onSuccess: () => void;
  onCancel: () => void;
}

const ComposeForm: Component<ComposeFormProps> = (props) => {
  const { t } = useI18n();
  const [to, setTo] = createSignal(props.replyTo?.sender.username || props.defaultRecipient || '');
  const [subject, setSubject] = createSignal(
    props.replyTo ? `Re: ${props.replyTo.subject}` : ''
  );
  const [body, setBody] = createSignal(
    props.replyTo
      ? `\n\n--- Original Message ---\n${props.replyTo.body}`
      : ''
  );
  const [error, setError] = createSignal('');
  const [loading, setLoading] = createSignal(false);

  const handleSubmit = async (e: Event) => {
    e.preventDefault();
    setError('');
    setLoading(true);

    try {
      await mailApi.sendMail({
        recipient: to(),
        subject: subject(),
        body: body(),
      });
      props.onSuccess();
    } catch (err: unknown) {
      if (err instanceof Error) {
        setError(err.message);
      } else {
        setError(t('mail.sendFailed'));
      }
    } finally {
      setLoading(false);
    }
  };

  return (
    <form onSubmit={handleSubmit} class="space-y-4">
      <Show when={error()}>
        <Alert type="error" onClose={() => setError('')}>
          {error()}
        </Alert>
      </Show>

      <Input
        label={t('mail.recipient')}
        value={to()}
        onInput={(e) => setTo(e.currentTarget.value)}
        required
      />

      <Input
        label={t('mail.subject')}
        value={subject()}
        onInput={(e) => setSubject(e.currentTarget.value)}
        required
      />

      <Textarea
        label={t('mail.body')}
        value={body()}
        onInput={(e) => setBody(e.currentTarget.value)}
        required
        rows={10}
      />

      <div class="flex justify-end space-x-3">
        <Button type="button" variant="secondary" onClick={props.onCancel}>
          {t('common.cancel')}
        </Button>
        <Button type="submit" variant="primary" loading={loading()}>
          {t('mail.send')}
        </Button>
      </div>
    </form>
  );
};

function formatDate(dateStr: string): string {
  const date = new Date(dateStr);
  return date.toLocaleString('ja-JP', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  });
}
