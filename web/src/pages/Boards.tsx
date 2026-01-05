import { type Component, createResource, createSignal, For, Show } from 'solid-js';
import { A, useParams } from '@solidjs/router';
import { PageLoading, Pagination, Button, Input, Textarea, Modal, Alert, Empty } from '../components';
import * as boardApi from '../api/board';

// Board List Page
export const BoardsPage: Component = () => {
  const [boards] = createResource(boardApi.getBoards);

  return (
    <div class="space-y-6">
      <h1 class="text-2xl font-display font-bold text-neon-cyan">掲示板</h1>

      <Show when={!boards.loading} fallback={<PageLoading />}>
        <Show
          when={boards() && boards()!.length > 0}
          fallback={
            <Empty
              title="掲示板がありません"
              description="まだ掲示板が作成されていません"
            />
          }
        >
          <div class="space-y-2">
            <For each={boards()}>
              {(board) => (
                <A
                  href={`/boards/${board.id}`}
                  class="card-hover block"
                >
                  <div class="flex items-center justify-between">
                    <div>
                      <h3 class="font-medium text-gray-200">{board.name}</h3>
                      <Show when={board.description}>
                        <p class="text-sm text-gray-500 mt-1">{board.description}</p>
                      </Show>
                    </div>
                    <div class="flex items-center space-x-4 text-xs text-gray-500">
                      <span class="badge-cyan">
                        {board.board_type === 'thread' ? 'スレッド' : 'フラット'}
                      </span>
                      <span>{board.thread_count} スレッド</span>
                      <span>{board.post_count} 投稿</span>
                    </div>
                  </div>
                </A>
              )}
            </For>
          </div>
        </Show>
      </Show>
    </div>
  );
};

// Board Detail Page (Thread list or Flat posts)
export const BoardDetailPage: Component = () => {
  const params = useParams<{ id: string }>();
  const [page, setPage] = createSignal(1);
  const [showNewThread, setShowNewThread] = createSignal(false);

  const boardId = () => parseInt(params.id);

  const [board] = createResource(boardId, boardApi.getBoard);

  const [threads, { refetch }] = createResource(
    () => ({ boardId: boardId(), page: page() }),
    ({ boardId, page }) => boardApi.getThreads(boardId, { page, per_page: 20 })
  );

  const handlePageChange = (newPage: number) => {
    setPage(newPage);
  };

  const handleThreadCreated = () => {
    setShowNewThread(false);
    refetch();
  };

  return (
    <div class="space-y-6">
      <Show when={!board.loading && board()} fallback={<PageLoading />}>
        {/* Header */}
        <div class="flex items-center justify-between">
          <div>
            <div class="flex items-center space-x-2 text-sm text-gray-500 mb-2">
              <A href="/boards" class="hover:text-neon-cyan transition-colors">掲示板</A>
              <span>/</span>
            </div>
            <h1 class="text-2xl font-display font-bold text-neon-cyan">{board()!.name}</h1>
            <Show when={board()!.description}>
              <p class="text-gray-500 mt-1">{board()!.description}</p>
            </Show>
          </div>
          <Show when={board()!.can_post}>
            <Button variant="primary" onClick={() => setShowNewThread(true)}>
              新規スレッド
            </Button>
          </Show>
        </div>

        {/* Thread List */}
        <Show when={!threads.loading} fallback={<PageLoading />}>
          <Show
            when={threads()?.data && threads()!.data.length > 0}
            fallback={
              <Empty
                title="スレッドがありません"
                description="最初のスレッドを作成してください"
                action={
                  <Show when={board()!.can_post}>
                    <Button variant="primary" onClick={() => setShowNewThread(true)}>
                      スレッドを作成
                    </Button>
                  </Show>
                }
              />
            }
          >
            <div class="space-y-2">
              <For each={threads()!.data}>
                {(thread) => (
                  <A
                    href={`/threads/${thread.id}`}
                    class="card-hover block"
                  >
                    <div class="flex items-center justify-between">
                      <div>
                        <h3 class="font-medium text-gray-200">{thread.title}</h3>
                        <p class="text-xs text-gray-500 mt-1">
                          {thread.author.nickname} - {formatDate(thread.created_at)}
                        </p>
                      </div>
                      <div class="text-sm text-gray-500">
                        {thread.post_count} 件
                      </div>
                    </div>
                  </A>
                )}
              </For>
            </div>

            <Pagination
              page={threads()!.page}
              totalPages={threads()!.total_pages}
              onPageChange={handlePageChange}
            />
          </Show>
        </Show>

        {/* New Thread Modal */}
        <Modal
          isOpen={showNewThread()}
          onClose={() => setShowNewThread(false)}
          title="新規スレッド"
          size="lg"
        >
          <NewThreadForm
            boardId={boardId()}
            onSuccess={handleThreadCreated}
            onCancel={() => setShowNewThread(false)}
          />
        </Modal>
      </Show>
    </div>
  );
};

// Thread Detail Page
export const ThreadDetailPage: Component = () => {
  const params = useParams<{ id: string }>();
  const [page, setPage] = createSignal(1);

  const threadId = () => parseInt(params.id);

  const [thread] = createResource(threadId, boardApi.getThread);

  const [posts, { refetch }] = createResource(
    () => ({ threadId: threadId(), page: page() }),
    ({ threadId, page }) => boardApi.getPosts(threadId, { page, per_page: 50 })
  );

  const handlePageChange = (newPage: number) => {
    setPage(newPage);
  };

  const handlePostCreated = () => {
    refetch();
  };

  return (
    <div class="space-y-6">
      <Show when={!thread.loading && thread()} fallback={<PageLoading />}>
        {/* Header */}
        <div>
          <div class="flex items-center space-x-2 text-sm text-gray-500 mb-2">
            <A href="/boards" class="hover:text-neon-cyan transition-colors">掲示板</A>
            <span>/</span>
            <A href={`/boards/${thread()!.board_id}`} class="hover:text-neon-cyan transition-colors">
              戻る
            </A>
            <span>/</span>
          </div>
          <h1 class="text-2xl font-display font-bold text-neon-cyan">{thread()!.title}</h1>
          <p class="text-sm text-gray-500 mt-1">
            {thread()!.author.nickname} - {formatDate(thread()!.created_at)}
          </p>
        </div>

        {/* Posts */}
        <Show when={!posts.loading} fallback={<PageLoading />}>
          <div class="space-y-4">
            <For each={posts()?.data}>
              {(post, index) => (
                <div class="card">
                  <div class="flex items-start justify-between mb-2">
                    <div class="flex items-center space-x-2">
                      <span class="badge-cyan">
                        {(posts()!.page - 1) * posts()!.per_page + index() + 1}
                      </span>
                      <span class="font-medium text-gray-300">{post.author.nickname}</span>
                    </div>
                    <span class="text-xs text-gray-500">{formatDate(post.created_at)}</span>
                  </div>
                  <div class="text-gray-300 whitespace-pre-wrap">{post.content}</div>
                </div>
              )}
            </For>
          </div>

          <Show when={posts()}>
            <Pagination
              page={posts()!.page}
              totalPages={posts()!.total_pages}
              onPageChange={handlePageChange}
            />
          </Show>

          {/* Reply Form */}
          <div class="card">
            <h3 class="text-lg font-medium text-neon-cyan mb-4">返信</h3>
            <ReplyForm
              threadId={threadId()}
              onSuccess={handlePostCreated}
            />
          </div>
        </Show>
      </Show>
    </div>
  );
};

// New Thread Form Component
interface NewThreadFormProps {
  boardId: number;
  onSuccess: () => void;
  onCancel: () => void;
}

const NewThreadForm: Component<NewThreadFormProps> = (props) => {
  const [title, setTitle] = createSignal('');
  const [content, setContent] = createSignal('');
  const [error, setError] = createSignal('');
  const [loading, setLoading] = createSignal(false);

  const handleSubmit = async (e: Event) => {
    e.preventDefault();
    setError('');
    setLoading(true);

    try {
      await boardApi.createThread(props.boardId, {
        title: title(),
        content: content(),
      });
      props.onSuccess();
    } catch (err: any) {
      setError(err.message || 'スレッドの作成に失敗しました');
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
        label="タイトル"
        value={title()}
        onInput={(e) => setTitle(e.currentTarget.value)}
        required
        maxLength={50}
      />

      <Textarea
        label="本文"
        value={content()}
        onInput={(e) => setContent(e.currentTarget.value)}
        required
        rows={8}
      />

      <div class="flex justify-end space-x-3">
        <Button type="button" variant="secondary" onClick={props.onCancel}>
          キャンセル
        </Button>
        <Button type="submit" variant="primary" loading={loading()}>
          作成
        </Button>
      </div>
    </form>
  );
};

// Reply Form Component
interface ReplyFormProps {
  threadId: number;
  onSuccess: () => void;
}

const ReplyForm: Component<ReplyFormProps> = (props) => {
  const [content, setContent] = createSignal('');
  const [error, setError] = createSignal('');
  const [loading, setLoading] = createSignal(false);

  const handleSubmit = async (e: Event) => {
    e.preventDefault();
    if (!content().trim()) return;

    setError('');
    setLoading(true);

    try {
      await boardApi.createPost(props.threadId, { content: content() });
      setContent('');
      props.onSuccess();
    } catch (err: any) {
      setError(err.message || '投稿に失敗しました');
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

      <Textarea
        value={content()}
        onInput={(e) => setContent(e.currentTarget.value)}
        placeholder="返信を入力..."
        rows={4}
      />

      <div class="flex justify-end">
        <Button type="submit" variant="primary" loading={loading()}>
          投稿
        </Button>
      </div>
    </form>
  );
};

// Helper function
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
