import { type Component, createResource, createSignal, For, Show } from 'solid-js';
import { PageLoading, Pagination, Button, Input, Textarea, Select, Modal, Alert, Empty } from '../components';
import * as adminApi from '../api/admin';
import type { AdminUser, AdminBoard, AdminFolder } from '../types';
import { useI18n } from '../stores/i18n';

export const AdminPage: Component = () => {
  const { t } = useI18n();
  const [activeTab, setActiveTab] = createSignal<'users' | 'boards' | 'folders'>('users');

  return (
    <div class="space-y-6">
      <h1 class="text-2xl font-display font-bold text-neon-cyan">{t('admin.title')}</h1>

      {/* Tabs */}
      <div class="flex space-x-1 border-b border-neon-cyan/20">
        <TabButton active={activeTab() === 'users'} onClick={() => setActiveTab('users')}>
          {t('admin.users')}
        </TabButton>
        <TabButton active={activeTab() === 'boards'} onClick={() => setActiveTab('boards')}>
          {t('admin.boards')}
        </TabButton>
        <TabButton active={activeTab() === 'folders'} onClick={() => setActiveTab('folders')}>
          {t('admin.folders')}
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
  const { t } = useI18n();
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
          fallback={<Empty title={t('admin.noUsers')} />}
        >
          <div class="overflow-x-auto">
            <table class="w-full text-sm">
              <thead>
                <tr class="border-b border-neon-cyan/20">
                  <th class="text-left py-3 px-4 text-gray-400 font-medium">{t('admin.id')}</th>
                  <th class="text-left py-3 px-4 text-gray-400 font-medium">{t('admin.username')}</th>
                  <th class="text-left py-3 px-4 text-gray-400 font-medium">{t('admin.nickname')}</th>
                  <th class="text-left py-3 px-4 text-gray-400 font-medium">{t('admin.role')}</th>
                  <th class="text-left py-3 px-4 text-gray-400 font-medium">{t('admin.status')}</th>
                  <th class="text-right py-3 px-4 text-gray-400 font-medium">{t('admin.actions')}</th>
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
                          {t(`roles.${user.role}` as any)}
                        </span>
                      </td>
                      <td class="py-3 px-4">
                        <span class={user.is_active ? 'text-neon-green' : 'text-neon-pink'}>
                          {user.is_active ? t('admin.active') : t('admin.inactive')}
                        </span>
                      </td>
                      <td class="py-3 px-4 text-right">
                        <Button
                          variant="secondary"
                          onClick={() => setEditUser(user)}
                          class="text-xs"
                        >
                          {t('common.edit')}
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
        title={t('admin.editUser')}
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
  const { t } = useI18n();
  const [showCreate, setShowCreate] = createSignal(false);
  const [editBoard, setEditBoard] = createSignal<AdminBoard | null>(null);

  const [boards, { refetch }] = createResource(adminApi.getAdminBoards);

  const handleSuccess = () => {
    setShowCreate(false);
    setEditBoard(null);
    refetch();
  };

  const handleDelete = async (id: number) => {
    if (!confirm(t('admin.confirmDeleteBoard'))) return;
    await adminApi.deleteBoard(id);
    refetch();
  };

  return (
    <div class="space-y-4">
      <div class="flex justify-end">
        <Button variant="primary" onClick={() => setShowCreate(true)}>
          {t('admin.createBoard')}
        </Button>
      </div>

      <Show when={!boards.loading} fallback={<PageLoading />}>
        <Show
          when={boards() && boards()!.length > 0}
          fallback={<Empty title={t('admin.noBoards')} />}
        >
          <div class="space-y-2">
            <For each={boards()}>
              {(board) => (
                <div class="card">
                  <div class="flex items-center justify-between">
                    <div>
                      <h3 class="font-medium text-gray-200">{board.name}</h3>
                      <p class="text-sm text-gray-500 mt-1">
                        {board.board_type === 'thread' ? t('admin.threadType') : t('admin.flatType')} | {t('admin.readPermission')}: {t(`roles.${board.min_read_role}` as any)} | {t('admin.writePermission')}: {t(`roles.${board.min_write_role}` as any)} | {t('admin.paging')}: {board.disable_paging ? t('admin.pagingOff') : t('admin.pagingOn')}
                      </p>
                    </div>
                    <div class="flex space-x-2">
                      <Button
                        variant="secondary"
                        onClick={() => setEditBoard(board)}
                        class="text-xs"
                      >
                        {t('common.edit')}
                      </Button>
                      <Button
                        variant="danger"
                        onClick={() => handleDelete(board.id)}
                        class="text-xs"
                      >
                        {t('common.delete')}
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
        title={editBoard() ? t('admin.editBoard') : t('admin.createBoard')}
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
  const { t } = useI18n();
  const [showCreate, setShowCreate] = createSignal(false);
  const [editFolder, setEditFolder] = createSignal<AdminFolder | null>(null);

  const [folders, { refetch }] = createResource(adminApi.getAdminFolders);

  const handleSuccess = () => {
    setShowCreate(false);
    setEditFolder(null);
    refetch();
  };

  const handleDelete = async (id: number) => {
    if (!confirm(t('admin.confirmDeleteFolder'))) return;
    await adminApi.deleteFolder(id);
    refetch();
  };

  return (
    <div class="space-y-4">
      <div class="flex justify-end">
        <Button variant="primary" onClick={() => setShowCreate(true)}>
          {t('admin.createFolder')}
        </Button>
      </div>

      <Show when={!folders.loading} fallback={<PageLoading />}>
        <Show
          when={folders() && folders()!.length > 0}
          fallback={<Empty title={t('admin.noFolders')} />}
        >
          <div class="space-y-2">
            <For each={folders()}>
              {(folder) => (
                <div class="card">
                  <div class="flex items-center justify-between">
                    <div>
                      <h3 class="font-medium text-gray-200">{folder.name}</h3>
                      <p class="text-sm text-gray-500 mt-1">
                        {t('admin.readPermission')}: {t(`roles.${folder.permission}` as any)} | {t('admin.uploadPermission')}: {t(`roles.${folder.upload_perm}` as any)}
                      </p>
                    </div>
                    <div class="flex space-x-2">
                      <Button
                        variant="secondary"
                        onClick={() => setEditFolder(folder)}
                        class="text-xs"
                      >
                        {t('common.edit')}
                      </Button>
                      <Button
                        variant="danger"
                        onClick={() => handleDelete(folder.id)}
                        class="text-xs"
                      >
                        {t('common.delete')}
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
        title={editFolder() ? t('admin.editFolder') : t('admin.createFolder')}
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
  const { t } = useI18n();
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
      setError(err.message || t('admin.updateFailed'));
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
        {t('admin.username')}: {props.user.username}
      </div>

      <Input
        label={t('admin.nickname')}
        value={nickname()}
        onInput={(e) => setNickname(e.currentTarget.value)}
        required
      />

      <Input
        label={t('auth.email')}
        type="email"
        value={email()}
        onInput={(e) => setEmail(e.currentTarget.value)}
      />

      <Select
        label={t('admin.role')}
        value={role()}
        onChange={(e) => setRole(e.currentTarget.value)}
        options={[
          { value: 'guest', label: t('roles.guest') },
          { value: 'member', label: t('roles.member') },
          { value: 'subop', label: t('roles.subop') },
          { value: 'sysop', label: t('roles.sysop') },
        ]}
      />

      <label class="flex items-center space-x-2">
        <input
          type="checkbox"
          checked={isActive()}
          onChange={(e) => setIsActive(e.currentTarget.checked)}
          class="form-checkbox"
        />
        <span class="text-sm text-gray-400">{t('admin.active')}</span>
      </label>

      <div class="flex justify-end space-x-3">
        <Button type="button" variant="secondary" onClick={props.onCancel}>
          {t('common.cancel')}
        </Button>
        <Button type="submit" variant="primary" loading={loading()}>
          {t('common.update')}
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
  const { t } = useI18n();
  const [name, setName] = createSignal(props.board?.name || '');
  const [description, setDescription] = createSignal(props.board?.description || '');
  const [boardType, setBoardType] = createSignal(props.board?.board_type || 'thread');
  const [minReadRole, setMinReadRole] = createSignal(props.board?.min_read_role || 'guest');
  const [minWriteRole, setMinWriteRole] = createSignal(props.board?.min_write_role || 'member');
  const [disablePaging, setDisablePaging] = createSignal(props.board?.disable_paging || false);
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
          disable_paging: disablePaging(),
        });
      } else {
        await adminApi.createBoard({
          name: name(),
          description: description() || undefined,
          board_type: boardType(),
          min_read_role: minReadRole(),
          min_write_role: minWriteRole(),
          disable_paging: disablePaging(),
        });
      }
      props.onSuccess();
    } catch (err: any) {
      setError(err.message || t('admin.operationFailed'));
    } finally {
      setLoading(false);
    }
  };

  const permissionOptions = [
    { value: 'guest', label: t('roles.guest') },
    { value: 'member', label: t('roles.member') },
    { value: 'subop', label: t('roles.subop') },
    { value: 'sysop', label: t('roles.sysop') },
  ];

  return (
    <form onSubmit={handleSubmit} class="space-y-4">
      <Show when={error()}>
        <Alert type="error" onClose={() => setError('')}>{error()}</Alert>
      </Show>

      <Input
        label={t('admin.name')}
        value={name()}
        onInput={(e) => setName(e.currentTarget.value)}
        required
      />

      <Textarea
        label={t('admin.description')}
        value={description()}
        onInput={(e) => setDescription(e.currentTarget.value)}
        rows={3}
      />

      <Show when={!props.board}>
        <Select
          label={t('admin.boardType')}
          value={boardType()}
          onChange={(e) => setBoardType(e.currentTarget.value)}
          options={[
            { value: 'thread', label: t('admin.threadType') },
            { value: 'flat', label: t('admin.flatType') },
          ]}
        />
      </Show>

      <Select
        label={t('admin.readPermission')}
        value={minReadRole()}
        onChange={(e) => setMinReadRole(e.currentTarget.value)}
        options={permissionOptions}
      />

      <Select
        label={t('admin.writePermission')}
        value={minWriteRole()}
        onChange={(e) => setMinWriteRole(e.currentTarget.value)}
        options={permissionOptions}
      />

      <label class="flex items-center space-x-2">
        <input
          type="checkbox"
          checked={disablePaging()}
          onChange={(e) => setDisablePaging(e.currentTarget.checked)}
          class="form-checkbox"
        />
        <span class="text-sm text-gray-400">
          {t('admin.paging')}: {disablePaging() ? t('admin.pagingOff') : t('admin.pagingOn')}
        </span>
      </label>

      <div class="flex justify-end space-x-3">
        <Button type="button" variant="secondary" onClick={props.onCancel}>
          {t('common.cancel')}
        </Button>
        <Button type="submit" variant="primary" loading={loading()}>
          {props.board ? t('common.update') : t('common.create')}
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
  const { t } = useI18n();
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
      setError(err.message || t('admin.operationFailed'));
    } finally {
      setLoading(false);
    }
  };

  const permissionOptions = [
    { value: 'guest', label: t('roles.guest') },
    { value: 'member', label: t('roles.member') },
    { value: 'subop', label: t('roles.subop') },
    { value: 'sysop', label: t('roles.sysop') },
  ];

  return (
    <form onSubmit={handleSubmit} class="space-y-4">
      <Show when={error()}>
        <Alert type="error" onClose={() => setError('')}>{error()}</Alert>
      </Show>

      <Input
        label={t('admin.name')}
        value={name()}
        onInput={(e) => setName(e.currentTarget.value)}
        required
      />

      <Textarea
        label={t('admin.description')}
        value={description()}
        onInput={(e) => setDescription(e.currentTarget.value)}
        rows={3}
      />

      <Select
        label={t('admin.readPermission')}
        value={permission()}
        onChange={(e) => setPermission(e.currentTarget.value)}
        options={permissionOptions}
      />

      <Select
        label={t('admin.uploadPermission')}
        value={uploadPerm()}
        onChange={(e) => setUploadPerm(e.currentTarget.value)}
        options={permissionOptions}
      />

      <div class="flex justify-end space-x-3">
        <Button type="button" variant="secondary" onClick={props.onCancel}>
          {t('common.cancel')}
        </Button>
        <Button type="submit" variant="primary" loading={loading()}>
          {props.folder ? t('common.update') : t('common.create')}
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
