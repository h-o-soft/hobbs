import { type Component, Show, lazy, Suspense } from 'solid-js';
import { Router, Route, Navigate } from '@solidjs/router';
import { AuthProvider, useAuth } from './stores/auth';
import { I18nProvider } from './stores/i18n';
import { Layout, PageLoading } from './components';

// Lazy load pages
const LoginPage = lazy(() => import('./pages/Login').then(m => ({ default: m.LoginPage })));
const RegisterPage = lazy(() => import('./pages/Register').then(m => ({ default: m.RegisterPage })));
const HomePage = lazy(() => import('./pages/Home').then(m => ({ default: m.HomePage })));
const BoardsPage = lazy(() => import('./pages/Boards').then(m => ({ default: m.BoardsPage })));
const BoardDetailPage = lazy(() => import('./pages/Boards').then(m => ({ default: m.BoardDetailPage })));
const ThreadDetailPage = lazy(() => import('./pages/Boards').then(m => ({ default: m.ThreadDetailPage })));
const MailPage = lazy(() => import('./pages/Mail').then(m => ({ default: m.MailPage })));
const ChatPage = lazy(() => import('./pages/Chat').then(m => ({ default: m.ChatPage })));
const FilesPage = lazy(() => import('./pages/Files').then(m => ({ default: m.FilesPage })));
const FolderDetailPage = lazy(() => import('./pages/Files').then(m => ({ default: m.FolderDetailPage })));
const RssPage = lazy(() => import('./pages/Rss').then(m => ({ default: m.RssPage })));
const RssDetailPage = lazy(() => import('./pages/Rss').then(m => ({ default: m.RssDetailPage })));
const AdminPage = lazy(() => import('./pages/Admin').then(m => ({ default: m.AdminPage })));
const ProfilePage = lazy(() => import('./pages/Profile').then(m => ({ default: m.ProfilePage })));
const ProfileEditPage = lazy(() => import('./pages/Profile').then(m => ({ default: m.ProfileEditPage })));
const UserProfilePage = lazy(() => import('./pages/Profile').then(m => ({ default: m.UserProfilePage })));

// Protected route wrapper
const ProtectedRoute: Component<{ children: any }> = (props) => {
  const [auth] = useAuth();

  return (
    <Show
      when={!auth.isLoading}
      fallback={<PageLoading />}
    >
      <Show
        when={auth.isAuthenticated}
        fallback={<Navigate href="/login" />}
      >
        <Layout>
          <Suspense fallback={<PageLoading />}>
            {props.children}
          </Suspense>
        </Layout>
      </Show>
    </Show>
  );
};

// Admin route wrapper
const AdminRoute: Component<{ children: any }> = (props) => {
  const [auth] = useAuth();

  return (
    <Show
      when={!auth.isLoading}
      fallback={<PageLoading />}
    >
      <Show
        when={auth.isAuthenticated && (auth.user?.role === 'sysop' || auth.user?.role === 'subop')}
        fallback={<Navigate href="/" />}
      >
        <Layout>
          <Suspense fallback={<PageLoading />}>
            {props.children}
          </Suspense>
        </Layout>
      </Show>
    </Show>
  );
};

// Public route wrapper (redirects to home if authenticated)
const PublicRoute: Component<{ children: any }> = (props) => {
  const [auth] = useAuth();

  return (
    <Show
      when={!auth.isLoading}
      fallback={<PageLoading />}
    >
      <Show
        when={!auth.isAuthenticated}
        fallback={<Navigate href="/" />}
      >
        <Suspense fallback={<PageLoading />}>
          {props.children}
        </Suspense>
      </Show>
    </Show>
  );
};

const App: Component = () => {
  return (
    <I18nProvider>
      <AuthProvider>
        <Router>
        {/* Public Routes */}
        <Route path="/login" component={() => (
          <PublicRoute>
            <LoginPage />
          </PublicRoute>
        )} />
        <Route path="/register" component={() => (
          <PublicRoute>
            <RegisterPage />
          </PublicRoute>
        )} />

        {/* Protected Routes */}
        <Route path="/" component={() => (
          <ProtectedRoute>
            <HomePage />
          </ProtectedRoute>
        )} />
        <Route path="/boards" component={() => (
          <ProtectedRoute>
            <BoardsPage />
          </ProtectedRoute>
        )} />
        <Route path="/boards/:id" component={() => (
          <ProtectedRoute>
            <BoardDetailPage />
          </ProtectedRoute>
        )} />
        <Route path="/threads/:id" component={() => (
          <ProtectedRoute>
            <ThreadDetailPage />
          </ProtectedRoute>
        )} />
        <Route path="/mail" component={() => (
          <ProtectedRoute>
            <MailPage />
          </ProtectedRoute>
        )} />
        <Route path="/chat" component={() => (
          <ProtectedRoute>
            <ChatPage />
          </ProtectedRoute>
        )} />
        <Route path="/files" component={() => (
          <ProtectedRoute>
            <FilesPage />
          </ProtectedRoute>
        )} />
        <Route path="/files/:id" component={() => (
          <ProtectedRoute>
            <FolderDetailPage />
          </ProtectedRoute>
        )} />
        <Route path="/rss" component={() => (
          <ProtectedRoute>
            <RssPage />
          </ProtectedRoute>
        )} />
        <Route path="/rss/:id" component={() => (
          <ProtectedRoute>
            <RssDetailPage />
          </ProtectedRoute>
        )} />

        {/* Profile Routes */}
        <Route path="/profile" component={() => (
          <ProtectedRoute>
            <ProfilePage />
          </ProtectedRoute>
        )} />
        <Route path="/profile/edit" component={() => (
          <ProtectedRoute>
            <ProfileEditPage />
          </ProtectedRoute>
        )} />
        <Route path="/users/:username" component={() => (
          <ProtectedRoute>
            <UserProfilePage />
          </ProtectedRoute>
        )} />

        {/* Admin Routes */}
        <Route path="/admin" component={() => (
          <AdminRoute>
            <AdminPage />
          </AdminRoute>
        )} />

        {/* Fallback */}
        <Route path="*" component={() => <Navigate href="/" />} />
        </Router>
      </AuthProvider>
    </I18nProvider>
  );
};

export default App;
