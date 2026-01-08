import { type Component, createResource, createSignal, For, Show } from 'solid-js';
import { A, useParams } from '@solidjs/router';
import { PageLoading, Pagination, Button, Input, Textarea, Modal, Alert, Empty, UserLink, AnsiText } from '../components';
import * as boardApi from '../api/board';
import { useI18n } from '../stores/i18n';
import { useAuth } from '../stores/auth';
import type { Post } from '../types';

// Board List Page
export const BoardsPage: Component = () => {
  const { t } = useI18n();
  const [boards] = createResource(boardApi.getBoards);

  return (
    <div class="space-y-6">
      <h1 class="text-2xl font-display font-bold text-neon-cyan">{t('boards.title')}</h1>

      <Show when={!boards.loading} fallback={<PageLoading />}>
        <Show
          when={boards() && boards()!.length > 0}
          fallback={
            <Empty
              title={t('boards.noBoards')}
              description={t('boards.noBoardsDesc')}
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
                        {board.board_type === 'thread' ? t('boards.threadType') : t('boards.flatType')}
                      </span>
                      <span>{board.post_count ?? 0} {t('home.posts')}</span>
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
  const { t } = useI18n();
  const [auth] = useAuth();
  const params = useParams<{ id: string }>();
  const [page, setPage] = createSignal(1);
  const [showNewThread, setShowNewThread] = createSignal(false);
  const [showNewPost, setShowNewPost] = createSignal(false);
  const [editingPost, setEditingPost] = createSignal<Post | null>(null);

  const boardId = () => parseInt(params.id);

  const canEditPost = (post: Post) => {
    if (!auth.user) return false;
    const isAuthor = post.author.id === auth.user.id;
    const isAdmin = auth.user.role === 'sysop' || auth.user.role === 'subop';
    return isAuthor || isAdmin;
  };

  const [board] = createResource(boardId, boardApi.getBoard);

  // スレッド形式の掲示板用
  const [threads, { refetch: refetchThreads }] = createResource(
    () => board()?.board_type === 'thread' ? { boardId: boardId(), page: page() } : null,
    ({ boardId, page }) => boardApi.getThreads(boardId, { page, per_page: 20 })
  );

  // フラット形式の掲示板用
  const [flatPosts, { refetch: refetchPosts }] = createResource(
    () => board()?.board_type === 'flat' ? { boardId: boardId(), page: page() } : null,
    ({ boardId, page }) => boardApi.getFlatPosts(boardId, { page, per_page: 50 })
  );

  const handlePageChange = (newPage: number) => {
    setPage(newPage);
  };

  const handleThreadCreated = () => {
    setShowNewThread(false);
    refetchThreads();
  };

  const handlePostCreated = () => {
    setShowNewPost(false);
    refetchPosts();
  };

  const handleDeletePost = async (postId: number) => {
    if (!confirm(t('boards.confirmDeletePost'))) return;
    try {
      await boardApi.deletePost(postId);
      refetchPosts();
    } catch (err) {
      alert(t('boards.deletePostFailed'));
    }
  };

  const handleEditSuccess = () => {
    setEditingPost(null);
    refetchPosts();
  };

  const isThreadBoard = () => board()?.board_type === 'thread';

  return (
    <div class="space-y-6">
      <Show when={!board.loading && board()} fallback={<PageLoading />}>
        {/* Header */}
        <div class="flex items-center justify-between">
          <div>
            <div class="flex items-center space-x-2 text-sm text-gray-500 mb-2">
              <A href="/boards" class="hover:text-neon-cyan transition-colors">{t('boards.title')}</A>
              <span>/</span>
            </div>
            <h1 class="text-2xl font-display font-bold text-neon-cyan">{board()!.name}</h1>
            <Show when={board()!.description}>
              <p class="text-gray-500 mt-1">{board()!.description}</p>
            </Show>
          </div>
          <Show when={board()!.can_write}>
            <Show
              when={isThreadBoard()}
              fallback={
                <Button variant="primary" onClick={() => setShowNewPost(true)}>
                  {t('boards.newPost')}
                </Button>
              }
            >
              <Button variant="primary" onClick={() => setShowNewThread(true)}>
                {t('boards.newThread')}
              </Button>
            </Show>
          </Show>
        </div>

        {/* Thread Board: Thread List */}
        <Show when={isThreadBoard()}>
          <Show when={!threads.loading} fallback={<PageLoading />}>
            <Show
              when={threads()?.data && threads()!.data.length > 0}
              fallback={
                <Empty
                  title={t('boards.noThreads')}
                  description={t('boards.noThreadsDesc')}
                  action={
                    <Show when={board()!.can_write}>
                      <Button variant="primary" onClick={() => setShowNewThread(true)}>
                        {t('boards.createThread')}
                      </Button>
                    </Show>
                  }
                />
              }
            >
              <div class="space-y-2 max-w-3xl mx-auto">
                <For each={threads()!.data}>
                  {(thread) => (
                    <A
                      href={`/threads/${thread.id}`}
                      class="card-hover block"
                    >
                      <div class="flex items-center justify-between">
                        <div>
                          <h3 class="font-bold text-gray-200">{thread.title}</h3>
                          <p class="text-xs text-gray-500 mt-1">
                            <span class="text-gray-400 font-light">{thread.author.nickname}</span>
                            <span class="mx-1">-</span>
                            {formatDate(thread.created_at)}
                          </p>
                        </div>
                        <div class="text-sm text-gray-500">
                          {thread.post_count} {t('boards.postCount')}
                        </div>
                      </div>
                    </A>
                  )}
                </For>
              </div>

              <Pagination
                page={threads()!.meta.page}
                totalPages={Math.ceil(threads()!.meta.total / threads()!.meta.per_page)}
                onPageChange={handlePageChange}
              />
            </Show>
          </Show>
        </Show>

        {/* Flat Board: Post List */}
        <Show when={!isThreadBoard()}>
          <Show when={!flatPosts.loading} fallback={<PageLoading />}>
            <Show
              when={flatPosts()?.data && flatPosts()!.data.length > 0}
              fallback={
                <Empty
                  title={t('boards.noPosts')}
                  description={t('boards.noPostsDesc')}
                  action={
                    <Show when={board()!.can_write}>
                      <Button variant="primary" onClick={() => setShowNewPost(true)}>
                        {t('boards.createPost')}
                      </Button>
                    </Show>
                  }
                />
              }
            >
              <div class="space-y-4 max-w-3xl mx-auto">
                <For each={flatPosts()!.data}>
                  {(post, index) => (
                    <div class="card">
                      <div class="flex items-start justify-between mb-2">
                        <div class="flex items-center space-x-2">
                          {/* 投稿番号（非表示） */}
                          <span class="badge-cyan hidden">
                            {(flatPosts()!.meta.page - 1) * flatPosts()!.meta.per_page + index() + 1}
                          </span>
                          <UserLink
                            username={post.author.username}
                            displayName={post.author.nickname}
                            class="text-sm font-light"
                          />
                        </div>
                        <div class="flex items-center space-x-2">
                          <span class="text-xs text-gray-500">{formatDate(post.created_at)}</span>
                          <Show when={canEditPost(post)}>
                            <PostActions
                              onEdit={() => setEditingPost(post)}
                              onDelete={() => handleDeletePost(post.id)}
                            />
                          </Show>
                        </div>
                      </div>
                      <Show when={post.title}>
                        <h3 class="font-bold text-neon-cyan mb-2">{post.title}</h3>
                      </Show>
                      <AnsiText text={post.body} class="text-gray-300" />
                    </div>
                  )}
                </For>
              </div>

              <Pagination
                page={flatPosts()!.meta.page}
                totalPages={Math.ceil(flatPosts()!.meta.total / flatPosts()!.meta.per_page)}
                onPageChange={handlePageChange}
              />
            </Show>
          </Show>
        </Show>

        {/* New Thread Modal */}
        <Modal
          isOpen={showNewThread()}
          onClose={() => setShowNewThread(false)}
          title={t('boards.newThread')}
          size="lg"
        >
          <NewThreadForm
            boardId={boardId()}
            onSuccess={handleThreadCreated}
            onCancel={() => setShowNewThread(false)}
          />
        </Modal>

        {/* New Flat Post Modal */}
        <Modal
          isOpen={showNewPost()}
          onClose={() => setShowNewPost(false)}
          title={t('boards.newPost')}
          size="lg"
        >
          <NewFlatPostForm
            boardId={boardId()}
            onSuccess={handlePostCreated}
            onCancel={() => setShowNewPost(false)}
          />
        </Modal>

        {/* Edit Post Modal */}
        <Modal
          isOpen={editingPost() !== null}
          onClose={() => setEditingPost(null)}
          title={t('boards.editPost')}
          size="lg"
        >
          <Show when={editingPost()}>
            {(post) => (
              <EditPostForm
                post={post()}
                onSuccess={handleEditSuccess}
                onCancel={() => setEditingPost(null)}
              />
            )}
          </Show>
        </Modal>
      </Show>
    </div>
  );
};

// Thread Detail Page
export const ThreadDetailPage: Component = () => {
  const { t } = useI18n();
  const [auth] = useAuth();
  const params = useParams<{ id: string }>();
  const [page, setPage] = createSignal(1);
  const [editingPost, setEditingPost] = createSignal<Post | null>(null);

  const threadId = () => parseInt(params.id);

  const canEditPost = (post: Post) => {
    if (!auth.user) return false;
    const isAuthor = post.author.id === auth.user.id;
    const isAdmin = auth.user.role === 'sysop' || auth.user.role === 'subop';
    return isAuthor || isAdmin;
  };

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

  const handleDeletePost = async (postId: number) => {
    if (!confirm(t('boards.confirmDeletePost'))) return;
    try {
      await boardApi.deletePost(postId);
      refetch();
    } catch (err) {
      alert(t('boards.deletePostFailed'));
    }
  };

  const handleEditSuccess = () => {
    setEditingPost(null);
    refetch();
  };

  return (
    <div class="space-y-6">
      <Show when={!thread.loading && thread()} fallback={<PageLoading />}>
        {/* Header */}
        <div>
          <div class="flex items-center space-x-2 text-sm text-gray-500 mb-2">
            <A href="/boards" class="hover:text-neon-cyan transition-colors">{t('boards.title')}</A>
            <span>/</span>
            <A href={`/boards/${thread()!.board_id}`} class="hover:text-neon-cyan transition-colors">
              {t('common.back')}
            </A>
            <span>/</span>
          </div>
          <h1 class="text-2xl font-display font-bold text-neon-cyan">{thread()!.title}</h1>
          <p class="text-sm text-gray-500 mt-1">
            <UserLink
              username={thread()!.author.username}
              displayName={thread()!.author.nickname}
            />
            {' '}- {formatDate(thread()!.created_at)}
          </p>
        </div>

        {/* Posts */}
        <Show when={!posts.loading} fallback={<PageLoading />}>
          <div class="space-y-4 max-w-3xl mx-auto">
            <For each={posts()?.data}>
              {(post, index) => (
                <div class="card">
                  <div class="flex items-start justify-between mb-2">
                    <div class="flex items-center space-x-2">
                      {/* 投稿番号（非表示） */}
                      <span class="badge-cyan hidden">
                        {(posts()!.meta.page - 1) * posts()!.meta.per_page + index() + 1}
                      </span>
                      <UserLink
                        username={post.author.username}
                        displayName={post.author.nickname}
                        class="text-sm font-light"
                      />
                    </div>
                    <div class="flex items-center space-x-2">
                      <span class="text-xs text-gray-500">{formatDate(post.created_at)}</span>
                      <Show when={canEditPost(post)}>
                        <PostActions
                          onEdit={() => setEditingPost(post)}
                          onDelete={() => handleDeletePost(post.id)}
                        />
                      </Show>
                    </div>
                  </div>
                  <AnsiText text={post.body} class="text-gray-300" />
                </div>
              )}
            </For>
          </div>

          <Show when={posts()}>
            <Pagination
              page={posts()!.meta.page}
              totalPages={Math.ceil(posts()!.meta.total / posts()!.meta.per_page)}
              onPageChange={handlePageChange}
            />
          </Show>

          {/* Reply Form */}
          <div class="card">
            <h3 class="text-lg font-medium text-neon-cyan mb-4">{t('boards.reply')}</h3>
            <ReplyForm
              threadId={threadId()}
              onSuccess={handlePostCreated}
            />
          </div>
        </Show>

        {/* Edit Post Modal */}
        <Modal
          isOpen={editingPost() !== null}
          onClose={() => setEditingPost(null)}
          title={t('boards.editPost')}
          size="lg"
        >
          <Show when={editingPost()}>
            {(post) => (
              <EditPostForm
                post={post()}
                onSuccess={handleEditSuccess}
                onCancel={() => setEditingPost(null)}
              />
            )}
          </Show>
        </Modal>
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
  const { t } = useI18n();
  const [title, setTitle] = createSignal('');
  const [body, setBody] = createSignal('');
  const [error, setError] = createSignal('');
  const [loading, setLoading] = createSignal(false);

  const handleSubmit = async (e: Event) => {
    e.preventDefault();
    setError('');
    setLoading(true);

    try {
      await boardApi.createThread(props.boardId, {
        title: title(),
        body: body(),
      });
      props.onSuccess();
    } catch (err: unknown) {
      if (err instanceof Error) {
        setError(err.message);
      } else {
        setError(t('boards.createThreadFailed'));
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
        label={t('boards.postTitle')}
        value={title()}
        onInput={(e) => setTitle(e.currentTarget.value)}
        required
        maxLength={50}
      />

      <Textarea
        label={t('boards.postBody')}
        value={body()}
        onInput={(e) => setBody(e.currentTarget.value)}
        required
        rows={8}
      />

      <div class="flex justify-end space-x-3">
        <Button type="button" variant="secondary" onClick={props.onCancel}>
          {t('common.cancel')}
        </Button>
        <Button type="submit" variant="primary" loading={loading()}>
          {t('common.create')}
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
  const { t } = useI18n();
  const [body, setBody] = createSignal('');
  const [error, setError] = createSignal('');
  const [loading, setLoading] = createSignal(false);

  const handleSubmit = async (e: Event) => {
    e.preventDefault();
    if (!body().trim()) return;

    setError('');
    setLoading(true);

    try {
      await boardApi.createPost(props.threadId, { body: body() });
      setBody('');
      props.onSuccess();
    } catch (err: unknown) {
      if (err instanceof Error) {
        setError(err.message);
      } else {
        setError(t('boards.createPostFailed'));
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

      <Textarea
        value={body()}
        onInput={(e) => setBody(e.currentTarget.value)}
        placeholder={t('boards.replyPlaceholder')}
        rows={4}
      />

      <div class="flex justify-end">
        <Button type="submit" variant="primary" loading={loading()}>
          {t('common.send')}
        </Button>
      </div>
    </form>
  );
};

// New Flat Post Form Component
interface NewFlatPostFormProps {
  boardId: number;
  onSuccess: () => void;
  onCancel: () => void;
}

const NewFlatPostForm: Component<NewFlatPostFormProps> = (props) => {
  const { t } = useI18n();
  const [title, setTitle] = createSignal('');
  const [body, setBody] = createSignal('');
  const [error, setError] = createSignal('');
  const [loading, setLoading] = createSignal(false);

  const handleSubmit = async (e: Event) => {
    e.preventDefault();
    setError('');
    setLoading(true);

    try {
      await boardApi.createFlatPost(props.boardId, {
        title: title(),
        body: body(),
      });
      props.onSuccess();
    } catch (err: unknown) {
      if (err instanceof Error) {
        setError(err.message);
      } else {
        setError(t('boards.createPostFailed'));
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
        label={t('boards.postTitle')}
        value={title()}
        onInput={(e) => setTitle(e.currentTarget.value)}
        required
        maxLength={50}
      />

      <Textarea
        label={t('boards.postBody')}
        value={body()}
        onInput={(e) => setBody(e.currentTarget.value)}
        required
        rows={8}
      />

      <div class="flex justify-end space-x-3">
        <Button type="button" variant="secondary" onClick={props.onCancel}>
          {t('common.cancel')}
        </Button>
        <Button type="submit" variant="primary" loading={loading()}>
          {t('common.send')}
        </Button>
      </div>
    </form>
  );
};

// Post Actions Dropdown
interface PostActionsProps {
  onEdit: () => void;
  onDelete: () => void;
}

const PostActions: Component<PostActionsProps> = (props) => {
  const { t } = useI18n();
  const [open, setOpen] = createSignal(false);

  return (
    <div class="relative">
      <button
        onClick={(e) => {
          e.stopPropagation();
          setOpen(!open());
        }}
        class="p-1 text-gray-500 hover:text-neon-cyan transition-colors"
      >
        <svg class="w-5 h-5" fill="currentColor" viewBox="0 0 20 20">
          <path d="M10 6a2 2 0 110-4 2 2 0 010 4zM10 12a2 2 0 110-4 2 2 0 010 4zM10 18a2 2 0 110-4 2 2 0 010 4z" />
        </svg>
      </button>
      <Show when={open()}>
        <div
          class="absolute right-0 mt-1 w-32 bg-cyber-dark border border-neon-cyan/30 rounded shadow-lg z-10"
          onClick={(e) => e.stopPropagation()}
        >
          <button
            onClick={() => {
              setOpen(false);
              props.onEdit();
            }}
            class="w-full px-3 py-2 text-left text-sm text-gray-300 hover:bg-neon-cyan/10 hover:text-neon-cyan transition-colors"
          >
            {t('common.edit')}
          </button>
          <button
            onClick={() => {
              setOpen(false);
              props.onDelete();
            }}
            class="w-full px-3 py-2 text-left text-sm text-gray-300 hover:bg-neon-pink/10 hover:text-neon-pink transition-colors"
          >
            {t('common.delete')}
          </button>
        </div>
      </Show>
    </div>
  );
};

// Edit Post Form Component
interface EditPostFormProps {
  post: Post;
  onSuccess: () => void;
  onCancel: () => void;
}

const EditPostForm: Component<EditPostFormProps> = (props) => {
  const { t } = useI18n();
  const [title, setTitle] = createSignal(props.post.title || '');
  const [body, setBody] = createSignal(props.post.body);
  const [error, setError] = createSignal('');
  const [loading, setLoading] = createSignal(false);

  const handleSubmit = async (e: Event) => {
    e.preventDefault();
    setError('');
    setLoading(true);

    try {
      await boardApi.updatePost(props.post.id, {
        title: title() || undefined,
        body: body(),
      });
      props.onSuccess();
    } catch (err: unknown) {
      if (err instanceof Error) {
        setError(err.message);
      } else {
        setError(t('boards.editPostFailed'));
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

      <Show when={props.post.title !== undefined}>
        <Input
          label={t('boards.postTitle')}
          value={title()}
          onInput={(e) => setTitle(e.currentTarget.value)}
          maxLength={50}
        />
      </Show>

      <Textarea
        label={t('boards.postBody')}
        value={body()}
        onInput={(e) => setBody(e.currentTarget.value)}
        required
        rows={8}
      />

      <div class="flex justify-end space-x-3">
        <Button type="button" variant="secondary" onClick={props.onCancel}>
          {t('common.cancel')}
        </Button>
        <Button type="submit" variant="primary" loading={loading()}>
          {t('common.save')}
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
