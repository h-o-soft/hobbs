import { type Component, createResource, createSignal, For, Show } from 'solid-js';
import { PageLoading, Pagination, Button, Input, Textarea, Modal, Alert, Empty } from '../components';
import * as mailApi from '../api/mail';
import type { MailListItem, Mail } from '../types';

export const MailPage: Component = () => {
  const [activeTab, setActiveTab] = createSignal<'inbox' | 'sent'>('inbox');
  const [page, setPage] = createSignal(1);
  const [selectedMail, setSelectedMail] = createSignal<Mail | null>(null);
  const [showCompose, setShowCompose] = createSignal(false);
  const [replyTo, setReplyTo] = createSignal<Mail | null>(null);

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
    if (!confirm('このメールを削除しますか？')) return;
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
    refetchSent();
  };

  return (
    <div class="space-y-6">
      {/* Header */}
      <div class="flex items-center justify-between">
        <h1 class="text-2xl font-display font-bold text-neon-cyan">メール</h1>
        <Button variant="primary" onClick={() => setShowCompose(true)}>
          新規作成
        </Button>
      </div>

      {/* Tabs */}
      <div class="flex space-x-1 border-b border-neon-cyan/20">
        <TabButton
          active={activeTab() === 'inbox'}
          onClick={() => handleTabChange('inbox')}
        >
          受信トレイ
        </TabButton>
        <TabButton
          active={activeTab() === 'sent'}
          onClick={() => handleTabChange('sent')}
        >
          送信済み
        </TabButton>
      </div>

      {/* Mail List */}
      <Show when={!isLoading()} fallback={<PageLoading />}>
        <Show
          when={currentList()?.data && currentList()!.data.length > 0}
          fallback={
            <Empty
              title={activeTab() === 'inbox' ? '受信メールがありません' : '送信済みメールがありません'}
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
                          ? `From: ${mail.from_user?.nickname}`
                          : `To: ${mail.to_user?.nickname}`}
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
        title="メール"
        size="lg"
      >
        <Show when={selectedMail()}>
          {(mail) => (
            <div class="space-y-4">
              <div class="border-b border-neon-cyan/20 pb-4">
                <h3 class="text-lg font-medium text-gray-200">{mail().subject}</h3>
                <div class="text-sm text-gray-500 mt-2 space-y-1">
                  <p>From: {mail().from_user.nickname} ({mail().from_user.username})</p>
                  <p>To: {mail().to_user.nickname} ({mail().to_user.username})</p>
                  <p>Date: {formatDate(mail().created_at)}</p>
                </div>
              </div>
              <div class="text-gray-300 whitespace-pre-wrap">
                {mail().content}
              </div>
              <div class="flex justify-end space-x-3 pt-4">
                <Button variant="danger" onClick={() => handleDelete(mail().id)}>
                  削除
                </Button>
                <Show when={activeTab() === 'inbox'}>
                  <Button variant="primary" onClick={() => handleReply(mail())}>
                    返信
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
        onClose={() => { setShowCompose(false); setReplyTo(null); }}
        title={replyTo() ? '返信' : '新規メール'}
        size="lg"
      >
        <ComposeForm
          replyTo={replyTo()}
          onSuccess={handleComposed}
          onCancel={() => { setShowCompose(false); setReplyTo(null); }}
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
  onSuccess: () => void;
  onCancel: () => void;
}

const ComposeForm: Component<ComposeFormProps> = (props) => {
  const [to, setTo] = createSignal(props.replyTo?.from_user.username || '');
  const [subject, setSubject] = createSignal(
    props.replyTo ? `Re: ${props.replyTo.subject}` : ''
  );
  const [content, setContent] = createSignal(
    props.replyTo
      ? `\n\n--- Original Message ---\n${props.replyTo.content}`
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
        to_username: to(),
        subject: subject(),
        content: content(),
      });
      props.onSuccess();
    } catch (err: any) {
      setError(err.message || 'メールの送信に失敗しました');
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
        label="宛先 (ユーザーID)"
        value={to()}
        onInput={(e) => setTo(e.currentTarget.value)}
        required
      />

      <Input
        label="件名"
        value={subject()}
        onInput={(e) => setSubject(e.currentTarget.value)}
        required
      />

      <Textarea
        label="本文"
        value={content()}
        onInput={(e) => setContent(e.currentTarget.value)}
        required
        rows={10}
      />

      <div class="flex justify-end space-x-3">
        <Button type="button" variant="secondary" onClick={props.onCancel}>
          キャンセル
        </Button>
        <Button type="submit" variant="primary" loading={loading()}>
          送信
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
