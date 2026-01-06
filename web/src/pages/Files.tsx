import { type Component, createResource, createSignal, For, Show } from 'solid-js';
import { A, useParams } from '@solidjs/router';
import { PageLoading, Pagination, Button, Textarea, Modal, Alert, Empty, UserLink } from '../components';
import * as fileApi from '../api/file';
import { useI18n } from '../stores/i18n';

// Folder List Page
export const FilesPage: Component = () => {
  const { t } = useI18n();
  const [folders] = createResource(fileApi.getFolders);

  return (
    <div class="space-y-6">
      <h1 class="text-2xl font-display font-bold text-neon-cyan">{t('files.title')}</h1>

      <Show when={!folders.loading} fallback={<PageLoading />}>
        <Show
          when={folders() && folders()!.length > 0}
          fallback={
            <Empty
              title={t('files.noFolders')}
              description={t('files.noFoldersDesc')}
            />
          }
        >
          <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            <For each={folders()}>
              {(folder) => (
                <A
                  href={`/files/${folder.id}`}
                  class="card-hover"
                >
                  <div class="flex items-start space-x-3">
                    <div class="text-neon-purple">
                      <svg class="w-10 h-10" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
                      </svg>
                    </div>
                    <div class="flex-1 min-w-0">
                      <h3 class="font-medium text-gray-200 truncate">{folder.name}</h3>
                      <Show when={folder.description}>
                        <p class="text-sm text-gray-500 mt-1 truncate">{folder.description}</p>
                      </Show>
                      <p class="text-xs text-gray-600 mt-2">{folder.file_count} {t('files.title')}</p>
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

// Folder Detail Page (File List)
export const FolderDetailPage: Component = () => {
  const { t } = useI18n();
  const params = useParams<{ id: string }>();
  const [page, setPage] = createSignal(1);
  const [showUpload, setShowUpload] = createSignal(false);

  const folderId = () => parseInt(params.id);

  const [folder] = createResource(folderId, fileApi.getFolder);

  const [files, { refetch }] = createResource(
    () => ({ folderId: folderId(), page: page() }),
    ({ folderId, page }) => fileApi.getFiles(folderId, { page, per_page: 20 })
  );

  const handleUploadSuccess = () => {
    setShowUpload(false);
    refetch();
  };

  const handleDelete = async (fileId: number) => {
    if (!confirm(t('files.confirmDelete'))) return;
    await fileApi.deleteFile(fileId);
    refetch();
  };

  return (
    <div class="space-y-6">
      <Show when={!folder.loading && folder()} fallback={<PageLoading />}>
        {/* Header */}
        <div class="flex items-center justify-between">
          <div>
            <div class="flex items-center space-x-2 text-sm text-gray-500 mb-2">
              <A href="/files" class="hover:text-neon-cyan transition-colors">{t('files.title')}</A>
              <span>/</span>
            </div>
            <h1 class="text-2xl font-display font-bold text-neon-cyan">{folder()!.name}</h1>
            <Show when={folder()!.description}>
              <p class="text-gray-500 mt-1">{folder()!.description}</p>
            </Show>
          </div>
          <Show when={folder()!.can_upload}>
            <Button variant="primary" onClick={() => setShowUpload(true)}>
              {t('files.upload')}
            </Button>
          </Show>
        </div>

        {/* File List */}
        <Show when={!files.loading} fallback={<PageLoading />}>
          <Show
            when={files()?.data && files()!.data.length > 0}
            fallback={
              <Empty
                title={t('files.noFiles')}
                description={t('files.noFilesDesc')}
                action={
                  <Show when={folder()!.can_upload}>
                    <Button variant="primary" onClick={() => setShowUpload(true)}>
                      {t('files.upload')}
                    </Button>
                  </Show>
                }
              />
            }
          >
            <div class="space-y-2">
              <For each={files()!.data}>
                {(file) => (
                  <div class="card">
                    <div class="flex items-center justify-between">
                      <div class="flex items-center space-x-3 flex-1 min-w-0">
                        <div class="text-neon-cyan">
                          <svg class="w-8 h-8" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
                          </svg>
                        </div>
                        <div class="flex-1 min-w-0">
                          <h3 class="font-medium text-gray-200 truncate">{file.filename}</h3>
                          <div class="flex items-center space-x-4 text-xs text-gray-500 mt-1">
                            <span>{formatFileSize(file.size)}</span>
                            <UserLink
                              username={file.uploader.username}
                              displayName={file.uploader.nickname}
                            />
                            <span>{formatDate(file.created_at)}</span>
                            <span>{file.downloads} {t('files.downloads')}</span>
                          </div>
                          <Show when={file.description}>
                            <p class="text-sm text-gray-500 mt-1">{file.description}</p>
                          </Show>
                        </div>
                      </div>
                      <div class="flex items-center space-x-2 ml-4">
                        <a
                          href={fileApi.getDownloadUrl(file.id)}
                          class="btn-primary text-sm"
                          download={file.filename}
                        >
                          {t('files.download')}
                        </a>
                        <Button
                          variant="danger"
                          onClick={() => handleDelete(file.id)}
                          class="text-sm"
                        >
                          {t('common.delete')}
                        </Button>
                      </div>
                    </div>
                  </div>
                )}
              </For>
            </div>

            <Pagination
              page={files()!.meta.page}
              totalPages={Math.ceil(files()!.meta.total / files()!.meta.per_page)}
              onPageChange={setPage}
            />
          </Show>
        </Show>

        {/* Upload Modal */}
        <Modal
          isOpen={showUpload()}
          onClose={() => setShowUpload(false)}
          title={t('files.uploadTitle')}
        >
          <UploadForm
            folderId={folderId()}
            onSuccess={handleUploadSuccess}
            onCancel={() => setShowUpload(false)}
          />
        </Modal>
      </Show>
    </div>
  );
};

interface UploadFormProps {
  folderId: number;
  onSuccess: () => void;
  onCancel: () => void;
}

const UploadForm: Component<UploadFormProps> = (props) => {
  const { t } = useI18n();
  const [file, setFile] = createSignal<File | null>(null);
  const [description, setDescription] = createSignal('');
  const [error, setError] = createSignal('');
  const [loading, setLoading] = createSignal(false);

  const handleFileChange = (e: Event) => {
    const input = e.target as HTMLInputElement;
    if (input.files && input.files.length > 0) {
      setFile(input.files[0]);
    }
  };

  const handleSubmit = async (e: Event) => {
    e.preventDefault();
    const f = file();
    if (!f) return;

    setError('');
    setLoading(true);

    try {
      await fileApi.uploadFile(props.folderId, f, description() || undefined);
      props.onSuccess();
    } catch (err: any) {
      setError(err.message || t('files.uploadFailed'));
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

      <div class="space-y-1">
        <label class="block text-sm text-gray-400">{t('files.fileLabel')}</label>
        <input
          type="file"
          onChange={handleFileChange}
          required
          class="w-full text-sm text-gray-400 file:mr-4 file:py-2 file:px-4 file:rounded file:border-0 file:text-sm file:font-medium file:bg-neon-cyan/20 file:text-neon-cyan hover:file:bg-neon-cyan/30"
        />
      </div>

      <Show when={file()}>
        <div class="text-sm text-gray-400">
          {t('files.selected')}: {file()!.name} ({formatFileSize(file()!.size)})
        </div>
      </Show>

      <Textarea
        label={t('files.descriptionLabel')}
        value={description()}
        onInput={(e) => setDescription(e.currentTarget.value)}
        rows={3}
      />

      <div class="flex justify-end space-x-3">
        <Button type="button" variant="secondary" onClick={props.onCancel}>
          {t('common.cancel')}
        </Button>
        <Button type="submit" variant="primary" loading={loading()} disabled={!file()}>
          {t('files.upload')}
        </Button>
      </div>
    </form>
  );
};

function formatFileSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function formatDate(dateStr: string): string {
  const date = new Date(dateStr);
  return date.toLocaleDateString('ja-JP');
}
