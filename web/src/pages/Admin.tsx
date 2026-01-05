import { type Component, createResource, createSignal, For, Show } from 'solid-js';
import { PageLoading, Pagination, Button, Input, Textarea, Select, Modal, Alert, Empty } from '../components';
import * as adminApi from '../api/admin';
import type { AdminUser, AdminBoard, AdminFolder } from '../types';

export const AdminPage: Component = () => {
  const [activeTab, setActiveTab] = createSignal<'users' | 'boards' | 'folders'>('users');

  return (
    <div class="space-y-6">
      <h1 class="text-2xl font-display font-bold text-neon-cyan">管理</h1>

      {/* Tabs */}
      <div class="flex space-x-1 border-b border-neon-cyan/20">
        <TabButton active={activeTab() === 'users'} onClick={() => setActiveTab('users')}>
          ユーザー
        </TabButton>
        <TabButton active={activeTab() === 'boards'} onClick={() => setActiveTab('boards')}>
          掲示板
        </TabButton>
        <TabButton active={activeTab() === 'folders'} onClick={() => setActiveTab('folders')}>
          フォルダ
        </TabButton>
      </div>

      {/* Content */}
      <Show when={activeTab() === 'users'}>
        <UsersTab />
      </Show>
      <Show when={activeTab() === 'boards'}>
        <BoardsTab />
      </Show>
      <Show when={activeTab() === 'folders'}>
        <FoldersTab />
      </Show>
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

// Users Tab
const UsersTab: Component = () => {
  const [page, setPage] = createSignal(1);
  const [editUser, setEditUser] = createSignal<AdminUser | null>(null);

  const [users, { refetch }] = createResource(
    () => page(),
    (page) => adminApi.getUsers({ page, per_page: 20 })
  );

  const handleEditSuccess = () => {
    setEditUser(null);
    refetch();
  };

  return (
    <div class="space-y-4">
      <Show when={!users.loading} fallback={<PageLoading />}>
        <Show
          when={users()?.data && users()!.data.length > 0}
          fallback={<Empty title="ユーザーがいません" />}
        >
          <div class="overflow-x-auto">
            <table class="w-full text-sm">
              <thead>
                <tr class="border-b border-neon-cyan/20">
                  <th class="text-left py-3 px-4 text-gray-400 font-medium">ID</th>
                  <th class="text-left py-3 px-4 text-gray-400 font-medium">ユーザー名</th>
                  <th class="text-left py-3 px-4 text-gray-400 font-medium">ニックネーム</th>
                  <th class="text-left py-3 px-4 text-gray-400 font-medium">ロール</th>
                  <th class="text-left py-3 px-4 text-gray-400 font-medium">状態</th>
                  <th class="text-right py-3 px-4 text-gray-400 font-medium">操作</th>
                </tr>
              </thead>
              <tbody>
                <For each={users()!.data}>
                  {(user) => (
                    <tr class="border-b border-neon-cyan/10 hover:bg-neon-cyan/5">
                      <td class="py-3 px-4 text-gray-500">{user.id}</td>
                      <td class="py-3 px-4 text-gray-200">{user.username}</td>
                      <td class="py-3 px-4 text-gray-300">{user.nickname}</td>
                      <td class="py-3 px-4">
                        <span class={`badge-${getRoleBadgeColor(user.role)}`}>
                          {user.role}
                        </span>
                      </td>
                      <td class="py-3 px-4">
                        <span class={user.is_active ? 'text-neon-green' : 'text-neon-pink'}>
                          {user.is_active ? '有効' : '無効'}
                        </span>
                      </td>
                      <td class="py-3 px-4 text-right">
                        <Button
                          variant="secondary"
                          onClick={() => setEditUser(user)}
                          class="text-xs"
                        >
                          編集
                        </Button>
                      </td>
                    </tr>
                  )}
                </For>
              </tbody>
            </table>
          </div>

          <Pagination
            page={users()!.meta.page}
            totalPages={Math.ceil(users()!.meta.total / users()!.meta.per_page)}
            onPageChange={setPage}
          />
        </Show>
      </Show>

      {/* Edit User Modal */}
      <Modal
        isOpen={editUser() !== null}
        onClose={() => setEditUser(null)}
        title="ユーザー編集"
      >
        <Show when={editUser()}>
          {(user) => (
            <EditUserForm
              user={user()}
              onSuccess={handleEditSuccess}
              onCancel={() => setEditUser(null)}
            />
          )}
        </Show>
      </Modal>
    </div>
  );
};

// Boards Tab
const BoardsTab: Component = () => {
  const [showCreate, setShowCreate] = createSignal(false);
  const [editBoard, setEditBoard] = createSignal<AdminBoard | null>(null);

  const [boards, { refetch }] = createResource(adminApi.getAdminBoards);

  const handleSuccess = () => {
    setShowCreate(false);
    setEditBoard(null);
    refetch();
  };

  const handleDelete = async (id: number) => {
    if (!confirm('この掲示板を削除しますか？')) return;
    await adminApi.deleteBoard(id);
    refetch();
  };

  return (
    <div class="space-y-4">
      <div class="flex justify-end">
        <Button variant="primary" onClick={() => setShowCreate(true)}>
          新規作成
        </Button>
      </div>

      <Show when={!boards.loading} fallback={<PageLoading />}>
        <Show
          when={boards() && boards()!.length > 0}
          fallback={<Empty title="掲示板がありません" />}
        >
          <div class="space-y-2">
            <For each={boards()}>
              {(board) => (
                <div class="card">
                  <div class="flex items-center justify-between">
                    <div>
                      <h3 class="font-medium text-gray-200">{board.name}</h3>
                      <p class="text-sm text-gray-500 mt-1">
                        {board.board_type} | 閲覧: {board.min_read_role} | 投稿: {board.min_write_role}
                      </p>
                    </div>
                    <div class="flex space-x-2">
                      <Button
                        variant="secondary"
                        onClick={() => setEditBoard(board)}
                        class="text-xs"
                      >
                        編集
                      </Button>
                      <Button
                        variant="danger"
                        onClick={() => handleDelete(board.id)}
                        class="text-xs"
                      >
                        削除
                      </Button>
                    </div>
                  </div>
                </div>
              )}
            </For>
          </div>
        </Show>
      </Show>

      {/* Create/Edit Modal */}
      <Modal
        isOpen={showCreate() || editBoard() !== null}
        onClose={() => { setShowCreate(false); setEditBoard(null); }}
        title={editBoard() ? '掲示板編集' : '掲示板作成'}
      >
        <BoardForm
          board={editBoard()}
          onSuccess={handleSuccess}
          onCancel={() => { setShowCreate(false); setEditBoard(null); }}
        />
      </Modal>
    </div>
  );
};

// Folders Tab
const FoldersTab: Component = () => {
  const [showCreate, setShowCreate] = createSignal(false);
  const [editFolder, setEditFolder] = createSignal<AdminFolder | null>(null);

  const [folders, { refetch }] = createResource(adminApi.getAdminFolders);

  const handleSuccess = () => {
    setShowCreate(false);
    setEditFolder(null);
    refetch();
  };

  const handleDelete = async (id: number) => {
    if (!confirm('このフォルダを削除しますか？')) return;
    await adminApi.deleteFolder(id);
    refetch();
  };

  return (
    <div class="space-y-4">
      <div class="flex justify-end">
        <Button variant="primary" onClick={() => setShowCreate(true)}>
          新規作成
        </Button>
      </div>

      <Show when={!folders.loading} fallback={<PageLoading />}>
        <Show
          when={folders() && folders()!.length > 0}
          fallback={<Empty title="フォルダがありません" />}
        >
          <div class="space-y-2">
            <For each={folders()}>
              {(folder) => (
                <div class="card">
                  <div class="flex items-center justify-between">
                    <div>
                      <h3 class="font-medium text-gray-200">{folder.name}</h3>
                      <p class="text-sm text-gray-500 mt-1">
                        閲覧: {folder.permission} | アップロード: {folder.upload_perm}
                      </p>
                    </div>
                    <div class="flex space-x-2">
                      <Button
                        variant="secondary"
                        onClick={() => setEditFolder(folder)}
                        class="text-xs"
                      >
                        編集
                      </Button>
                      <Button
                        variant="danger"
                        onClick={() => handleDelete(folder.id)}
                        class="text-xs"
                      >
                        削除
                      </Button>
                    </div>
                  </div>
                </div>
              )}
            </For>
          </div>
        </Show>
      </Show>

      {/* Create/Edit Modal */}
      <Modal
        isOpen={showCreate() || editFolder() !== null}
        onClose={() => { setShowCreate(false); setEditFolder(null); }}
        title={editFolder() ? 'フォルダ編集' : 'フォルダ作成'}
      >
        <FolderForm
          folder={editFolder()}
          onSuccess={handleSuccess}
          onCancel={() => { setShowCreate(false); setEditFolder(null); }}
        />
      </Modal>
    </div>
  );
};

// Form Components
interface EditUserFormProps {
  user: AdminUser;
  onSuccess: () => void;
  onCancel: () => void;
}

const EditUserForm: Component<EditUserFormProps> = (props) => {
  const [nickname, setNickname] = createSignal(props.user.nickname);
  const [email, setEmail] = createSignal(props.user.email || '');
  const [role, setRole] = createSignal(props.user.role);
  const [isActive, setIsActive] = createSignal(props.user.is_active);
  const [error, setError] = createSignal('');
  const [loading, setLoading] = createSignal(false);

  const handleSubmit = async (e: Event) => {
    e.preventDefault();
    setError('');
    setLoading(true);

    try {
      // Update basic info
      await adminApi.updateUser(props.user.id, {
        nickname: nickname(),
        email: email() || undefined,
      });

      // Update role if changed
      if (role() !== props.user.role) {
        await adminApi.updateUserRole(props.user.id, role());
      }

      // Update status if changed
      if (isActive() !== props.user.is_active) {
        await adminApi.updateUserStatus(props.user.id, isActive());
      }

      props.onSuccess();
    } catch (err: any) {
      setError(err.message || '更新に失敗しました');
    } finally {
      setLoading(false);
    }
  };

  return (
    <form onSubmit={handleSubmit} class="space-y-4">
      <Show when={error()}>
        <Alert type="error" onClose={() => setError('')}>{error()}</Alert>
      </Show>

      <div class="text-sm text-gray-500">
        ユーザー名: {props.user.username}
      </div>

      <Input
        label="ニックネーム"
        value={nickname()}
        onInput={(e) => setNickname(e.currentTarget.value)}
        required
      />

      <Input
        label="メールアドレス"
        type="email"
        value={email()}
        onInput={(e) => setEmail(e.currentTarget.value)}
      />

      <Select
        label="ロール"
        value={role()}
        onChange={(e) => setRole(e.currentTarget.value)}
        options={[
          { value: 'guest', label: 'ゲスト' },
          { value: 'member', label: 'メンバー' },
          { value: 'subop', label: 'サブオペ' },
          { value: 'sysop', label: 'シスオペ' },
        ]}
      />

      <label class="flex items-center space-x-2">
        <input
          type="checkbox"
          checked={isActive()}
          onChange={(e) => setIsActive(e.currentTarget.checked)}
          class="form-checkbox"
        />
        <span class="text-sm text-gray-400">有効</span>
      </label>

      <div class="flex justify-end space-x-3">
        <Button type="button" variant="secondary" onClick={props.onCancel}>
          キャンセル
        </Button>
        <Button type="submit" variant="primary" loading={loading()}>
          更新
        </Button>
      </div>
    </form>
  );
};

interface BoardFormProps {
  board: AdminBoard | null;
  onSuccess: () => void;
  onCancel: () => void;
}

const BoardForm: Component<BoardFormProps> = (props) => {
  const [name, setName] = createSignal(props.board?.name || '');
  const [description, setDescription] = createSignal(props.board?.description || '');
  const [boardType, setBoardType] = createSignal(props.board?.board_type || 'thread');
  const [minReadRole, setMinReadRole] = createSignal(props.board?.min_read_role || 'guest');
  const [minWriteRole, setMinWriteRole] = createSignal(props.board?.min_write_role || 'member');
  const [error, setError] = createSignal('');
  const [loading, setLoading] = createSignal(false);

  const handleSubmit = async (e: Event) => {
    e.preventDefault();
    setError('');
    setLoading(true);

    try {
      if (props.board) {
        await adminApi.updateBoard(props.board.id, {
          name: name(),
          description: description() || undefined,
          min_read_role: minReadRole(),
          min_write_role: minWriteRole(),
        });
      } else {
        await adminApi.createBoard({
          name: name(),
          description: description() || undefined,
          board_type: boardType(),
          min_read_role: minReadRole(),
          min_write_role: minWriteRole(),
        });
      }
      props.onSuccess();
    } catch (err: any) {
      setError(err.message || '操作に失敗しました');
    } finally {
      setLoading(false);
    }
  };

  const permissionOptions = [
    { value: 'guest', label: 'ゲスト' },
    { value: 'member', label: 'メンバー' },
    { value: 'subop', label: 'サブオペ' },
    { value: 'sysop', label: 'シスオペ' },
  ];

  return (
    <form onSubmit={handleSubmit} class="space-y-4">
      <Show when={error()}>
        <Alert type="error" onClose={() => setError('')}>{error()}</Alert>
      </Show>

      <Input
        label="名前"
        value={name()}
        onInput={(e) => setName(e.currentTarget.value)}
        required
      />

      <Textarea
        label="説明"
        value={description()}
        onInput={(e) => setDescription(e.currentTarget.value)}
        rows={3}
      />

      <Show when={!props.board}>
        <Select
          label="タイプ"
          value={boardType()}
          onChange={(e) => setBoardType(e.currentTarget.value)}
          options={[
            { value: 'thread', label: 'スレッド形式' },
            { value: 'flat', label: 'フラット形式' },
          ]}
        />
      </Show>

      <Select
        label="閲覧権限"
        value={minReadRole()}
        onChange={(e) => setMinReadRole(e.currentTarget.value)}
        options={permissionOptions}
      />

      <Select
        label="投稿権限"
        value={minWriteRole()}
        onChange={(e) => setMinWriteRole(e.currentTarget.value)}
        options={permissionOptions}
      />

      <div class="flex justify-end space-x-3">
        <Button type="button" variant="secondary" onClick={props.onCancel}>
          キャンセル
        </Button>
        <Button type="submit" variant="primary" loading={loading()}>
          {props.board ? '更新' : '作成'}
        </Button>
      </div>
    </form>
  );
};

interface FolderFormProps {
  folder: AdminFolder | null;
  onSuccess: () => void;
  onCancel: () => void;
}

const FolderForm: Component<FolderFormProps> = (props) => {
  const [name, setName] = createSignal(props.folder?.name || '');
  const [description, setDescription] = createSignal(props.folder?.description || '');
  const [permission, setPermission] = createSignal(props.folder?.permission || 'member');
  const [uploadPerm, setUploadPerm] = createSignal(props.folder?.upload_perm || 'member');
  const [error, setError] = createSignal('');
  const [loading, setLoading] = createSignal(false);

  const handleSubmit = async (e: Event) => {
    e.preventDefault();
    setError('');
    setLoading(true);

    try {
      if (props.folder) {
        await adminApi.updateFolder(props.folder.id, {
          name: name(),
          description: description() || undefined,
          permission: permission(),
          upload_perm: uploadPerm(),
        });
      } else {
        await adminApi.createFolder({
          name: name(),
          description: description() || undefined,
          permission: permission(),
          upload_perm: uploadPerm(),
        });
      }
      props.onSuccess();
    } catch (err: any) {
      setError(err.message || '操作に失敗しました');
    } finally {
      setLoading(false);
    }
  };

  const permissionOptions = [
    { value: 'guest', label: 'ゲスト' },
    { value: 'member', label: 'メンバー' },
    { value: 'subop', label: 'サブオペ' },
    { value: 'sysop', label: 'シスオペ' },
  ];

  return (
    <form onSubmit={handleSubmit} class="space-y-4">
      <Show when={error()}>
        <Alert type="error" onClose={() => setError('')}>{error()}</Alert>
      </Show>

      <Input
        label="名前"
        value={name()}
        onInput={(e) => setName(e.currentTarget.value)}
        required
      />

      <Textarea
        label="説明"
        value={description()}
        onInput={(e) => setDescription(e.currentTarget.value)}
        rows={3}
      />

      <Select
        label="閲覧権限"
        value={permission()}
        onChange={(e) => setPermission(e.currentTarget.value)}
        options={permissionOptions}
      />

      <Select
        label="アップロード権限"
        value={uploadPerm()}
        onChange={(e) => setUploadPerm(e.currentTarget.value)}
        options={permissionOptions}
      />

      <div class="flex justify-end space-x-3">
        <Button type="button" variant="secondary" onClick={props.onCancel}>
          キャンセル
        </Button>
        <Button type="submit" variant="primary" loading={loading()}>
          {props.folder ? '更新' : '作成'}
        </Button>
      </div>
    </form>
  );
};

function getRoleBadgeColor(role: string): string {
  switch (role) {
    case 'sysop': return 'pink';
    case 'subop': return 'purple';
    case 'member': return 'cyan';
    default: return 'green';
  }
}
