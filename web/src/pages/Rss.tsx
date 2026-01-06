import { type Component, createResource, createSignal, For, Show } from 'solid-js';
import { useParams, useNavigate } from '@solidjs/router';
import { PageLoading, Pagination, Button, Input, Modal, Alert, Empty } from '../components';
import * as rssApi from '../api/rss';
import type { RssItem } from '../types';
import { useI18n } from '../stores/i18n';

// RSS Feed List Page
export const RssPage: Component = () => {
  const { t } = useI18n();
  const [showAddFeed, setShowAddFeed] = createSignal(false);
  const navigate = useNavigate();

  const [feeds, { refetch }] = createResource(rssApi.getFeeds);

  const handleAddSuccess = () => {
    setShowAddFeed(false);
    refetch();
  };

  const handleDeleteFeed = async (id: number, e: Event) => {
    e.stopPropagation();
    if (!confirm(t('rss.confirmDelete'))) return;
    await rssApi.deleteFeed(id);
    refetch();
  };

  return (
    <div class="space-y-6">
      {/* Header */}
      <div class="flex items-center justify-between">
        <h1 class="text-2xl font-display font-bold text-neon-cyan">{t('rss.title')}</h1>
        <Button variant="primary" onClick={() => setShowAddFeed(true)}>
          {t('rss.addFeed')}
        </Button>
      </div>

      {/* Feed List */}
      <Show when={!feeds.loading} fallback={<PageLoading />}>
        <Show
          when={feeds() && feeds()!.length > 0}
          fallback={
            <Empty
              title={t('rss.noFeeds')}
              description={t('rss.noFeedsDesc')}
              action={
                <Button variant="primary" onClick={() => setShowAddFeed(true)}>
                  {t('rss.addFeed')}
                </Button>
              }
            />
          }
        >
          <div class="space-y-2">
            <For each={feeds()}>
              {(feed) => (
                <div
                  onClick={() => navigate(`/rss/${feed.id}`)}
                  class="card-hover cursor-pointer"
                >
                  <div class="flex items-center justify-between">
                    <div class="flex-1 min-w-0">
                      <div class="flex items-center space-x-2">
                        <h3 class="font-medium text-gray-200 truncate">{feed.title}</h3>
                        <Show when={feed.unread_count > 0}>
                          <span class="badge-pink">{feed.unread_count} {t('rss.unread')}</span>
                        </Show>
                      </div>
                      <Show when={feed.description}>
                        <p class="text-sm text-gray-500 mt-1 truncate">{feed.description}</p>
                      </Show>
                      <p class="text-xs text-gray-600 mt-1 truncate">{feed.url}</p>
                    </div>
                    <button
                      onClick={(e) => handleDeleteFeed(feed.id, e)}
                      class="p-2 text-gray-500 hover:text-neon-pink transition-colors ml-4"
                      title={t('common.delete')}
                    >
                      <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                      </svg>
                    </button>
                  </div>
                </div>
              )}
            </For>
          </div>
        </Show>
      </Show>

      {/* Add Feed Modal */}
      <Modal
        isOpen={showAddFeed()}
        onClose={() => setShowAddFeed(false)}
        title={t('rss.addFeedTitle')}
      >
        <AddFeedForm
          onSuccess={handleAddSuccess}
          onCancel={() => setShowAddFeed(false)}
        />
      </Modal>
    </div>
  );
};

// RSS Feed Detail Page (Items)
export const RssDetailPage: Component = () => {
  const { t } = useI18n();
  const params = useParams<{ id: string }>();
  const navigate = useNavigate();
  const [page, setPage] = createSignal(1);
  const [selectedItem, setSelectedItem] = createSignal<RssItem | null>(null);

  const feedId = () => parseInt(params.id);

  const [feed] = createResource(feedId, rssApi.getFeed);

  const [items, { refetch }] = createResource(
    () => ({ feedId: feedId(), page: page() }),
    ({ feedId, page }) => rssApi.getItems(feedId, { page, per_page: 30 })
  );

  const handleItemClick = (item: RssItem) => {
    setSelectedItem(item);
  };

  const handleMarkAllRead = async () => {
    await rssApi.markAllAsRead(feedId());
    refetch();
  };

  return (
    <div class="space-y-6">
      <Show when={!feed.loading && feed()} fallback={<PageLoading />}>
        {/* Header */}
        <div class="flex items-center justify-between">
          <div>
            <button
              onClick={() => navigate('/rss')}
              class="text-sm text-gray-500 hover:text-neon-cyan transition-colors mb-2"
            >
              ← {t('rss.backToList')}
            </button>
            <h1 class="text-2xl font-display font-bold text-neon-cyan">{feed()!.title}</h1>
            <Show when={feed()!.description}>
              <p class="text-gray-500 mt-1">{feed()!.description}</p>
            </Show>
          </div>
          <Button variant="secondary" onClick={handleMarkAllRead}>
            {t('rss.markAllRead')}
          </Button>
        </div>

        {/* Items */}
        <Show when={!items.loading} fallback={<PageLoading />}>
          <Show
            when={items()?.data && items()!.data.length > 0}
            fallback={
              <Empty title={t('rss.noItems')} />
            }
          >
            <div class="space-y-2">
              <For each={items()!.data}>
                {(item) => (
                  <div
                    onClick={() => handleItemClick(item)}
                    class="card-hover cursor-pointer"
                  >
                    <div class="flex-1 min-w-0">
                      <h3 class="font-medium text-gray-200">{item.title}</h3>
                      <Show when={item.published_at}>
                        <p class="text-xs text-gray-500 mt-1">{formatDate(item.published_at!)}</p>
                      </Show>
                    </div>
                  </div>
                )}
              </For>
            </div>

            <Pagination
              page={items()!.meta.page}
              totalPages={Math.ceil(items()!.meta.total / items()!.meta.per_page)}
              onPageChange={setPage}
            />
          </Show>
        </Show>

        {/* Item Detail Modal */}
        <Modal
          isOpen={selectedItem() !== null}
          onClose={() => setSelectedItem(null)}
          title={selectedItem()?.title}
          size="lg"
        >
          <Show when={selectedItem()}>
            {(item) => (
              <div class="space-y-4">
                <Show when={item().published_at}>
                  <p class="text-sm text-gray-500">{formatDate(item().published_at!)}</p>
                </Show>
                <Show when={item().description}>
                  <div
                    class="text-gray-300 prose prose-invert prose-sm max-w-none"
                    innerHTML={item().description}
                  />
                </Show>
                <Show when={item().link}>
                  <a
                    href={item().link}
                    target="_blank"
                    rel="noopener noreferrer"
                    class="btn-primary inline-block"
                  >
                    {t('rss.readOriginal')}
                  </a>
                </Show>
              </div>
            )}
          </Show>
        </Modal>
      </Show>
    </div>
  );
};

interface AddFeedFormProps {
  onSuccess: () => void;
  onCancel: () => void;
}

const AddFeedForm: Component<AddFeedFormProps> = (props) => {
  const { t } = useI18n();
  const [url, setUrl] = createSignal('');
  const [title, setTitle] = createSignal('');
  const [error, setError] = createSignal('');
  const [loading, setLoading] = createSignal(false);

  const handleSubmit = async (e: Event) => {
    e.preventDefault();
    setError('');
    setLoading(true);

    try {
      await rssApi.addFeed({
        url: url(),
        title: title() || undefined,
      });
      props.onSuccess();
    } catch (err: any) {
      setError(err.message || t('rss.addFailed'));
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
        label={t('rss.url')}
        type="url"
        value={url()}
        onInput={(e) => setUrl(e.currentTarget.value)}
        required
        placeholder="https://example.com/feed.xml"
      />

      <Input
        label={t('rss.titleOptional')}
        value={title()}
        onInput={(e) => setTitle(e.currentTarget.value)}
        placeholder={t('rss.titleAutoFetch')}
      />

      <div class="flex justify-end space-x-3">
        <Button type="button" variant="secondary" onClick={props.onCancel}>
          {t('common.cancel')}
        </Button>
        <Button type="submit" variant="primary" loading={loading()}>
          {t('common.add')}
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
