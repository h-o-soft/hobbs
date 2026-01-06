import { type Component, createResource, createSignal, Show } from 'solid-js';
import { useNavigate, useParams } from '@solidjs/router';
import { PageLoading, Button, Input, Textarea, Alert } from '../components';
import * as userApi from '../api/user';
import { useAuth } from '../stores/auth';

/**
 * Profile page - Shows current user's own profile.
 */
export const ProfilePage: Component = () => {
  const navigate = useNavigate();

  const [profile] = createResource(async () => {
    return userApi.getMyProfile();
  });

  const formatDate = (dateStr?: string) => {
    if (!dateStr) return '-';
    return new Date(dateStr).toLocaleString('ja-JP');
  };

  const getRoleLabel = (role: string) => {
    switch (role) {
      case 'sysop': return 'SysOp';
      case 'subop': return 'SubOp';
      case 'member': return 'Member';
      case 'guest': return 'Guest';
      default: return role;
    }
  };

  return (
    <div class="space-y-6">
      {/* Header */}
      <div class="flex items-center justify-between">
        <h1 class="text-2xl font-display font-bold text-neon-cyan">
          My Profile
        </h1>
        <Button variant="primary" onClick={() => navigate('/profile/edit')}>
          Edit
        </Button>
      </div>

      {/* Profile Content */}
      <Show when={!profile.loading} fallback={<PageLoading />}>
        <Show when={profile()}>
          {(user) => (
            <div class="card space-y-6">
              {/* Basic Info */}
              <div class="space-y-4">
                <ProfileField label="Username" value={user().username} />
                <ProfileField label="Nickname" value={user().nickname} />
                <ProfileField label="Role" value={getRoleLabel(user().role)} />
                <ProfileField label="Registered" value={formatDate(user().created_at)} />
                <ProfileField label="Last Login" value={formatDate(user().last_login_at)} />
              </div>

              {/* Profile / Bio */}
              <div class="border-t border-neon-cyan/20 pt-4">
                <h3 class="text-sm font-medium text-gray-400 mb-2">Bio</h3>
                <div class="text-gray-200 whitespace-pre-wrap">
                  {user().profile || <span class="text-gray-500 italic">No bio set</span>}
                </div>
              </div>
            </div>
          )}
        </Show>
      </Show>
    </div>
  );
};

/**
 * Profile edit page - Allows editing own profile.
 */
export const ProfileEditPage: Component = () => {
  const navigate = useNavigate();
  const [, authActions] = useAuth();

  const [profile] = createResource(async () => {
    return userApi.getMyProfile();
  });

  const [nickname, setNickname] = createSignal('');
  const [bio, setBio] = createSignal('');
  const [currentPassword, setCurrentPassword] = createSignal('');
  const [newPassword, setNewPassword] = createSignal('');
  const [confirmPassword, setConfirmPassword] = createSignal('');
  const [saving, setSaving] = createSignal(false);
  const [error, setError] = createSignal('');
  const [success, setSuccess] = createSignal('');
  const [passwordError, setPasswordError] = createSignal('');
  const [passwordSuccess, setPasswordSuccess] = createSignal('');

  // Initialize form when profile loads
  const initForm = () => {
    const p = profile();
    if (p) {
      setNickname(p.nickname);
      setBio(p.profile || '');
    }
  };

  // Watch for profile load
  createResource(() => profile(), (p) => {
    if (p) initForm();
    return null;
  });

  const handleSaveProfile = async (e: Event) => {
    e.preventDefault();
    setError('');
    setSuccess('');
    setSaving(true);

    try {
      await userApi.updateMyProfile({
        nickname: nickname(),
        profile: bio(),
      });
      setSuccess('Profile updated successfully');
      await authActions.checkAuth(); // Refresh auth state
    } catch (err: any) {
      setError(err.message || 'Failed to update profile');
    } finally {
      setSaving(false);
    }
  };

  const handleChangePassword = async (e: Event) => {
    e.preventDefault();
    setPasswordError('');
    setPasswordSuccess('');

    if (newPassword() !== confirmPassword()) {
      setPasswordError('Passwords do not match');
      return;
    }

    if (newPassword().length < 8) {
      setPasswordError('Password must be at least 8 characters');
      return;
    }

    setSaving(true);
    try {
      await userApi.changePassword({
        current_password: currentPassword(),
        new_password: newPassword(),
      });
      setPasswordSuccess('Password changed successfully');
      setCurrentPassword('');
      setNewPassword('');
      setConfirmPassword('');
    } catch (err: any) {
      setPasswordError(err.message || 'Failed to change password');
    } finally {
      setSaving(false);
    }
  };

  return (
    <div class="space-y-6">
      {/* Header */}
      <div class="flex items-center justify-between">
        <h1 class="text-2xl font-display font-bold text-neon-cyan">
          Edit Profile
        </h1>
        <Button variant="secondary" onClick={() => navigate('/profile')}>
          Back
        </Button>
      </div>

      <Show when={!profile.loading} fallback={<PageLoading />}>
        <Show when={profile()}>
          {(user) => (
            <div class="space-y-8">
              {/* Profile Edit Form */}
              <form onSubmit={handleSaveProfile} class="card space-y-4">
                <h2 class="text-lg font-medium text-neon-cyan">Profile Information</h2>

                <Show when={error()}>
                  <Alert type="error">{error()}</Alert>
                </Show>
                <Show when={success()}>
                  <Alert type="success">{success()}</Alert>
                </Show>

                <Input
                  label="Username"
                  value={user().username}
                  disabled
                />

                <Input
                  label="Nickname"
                  value={nickname()}
                  onInput={(e) => setNickname(e.currentTarget.value)}
                  maxLength={20}
                  required
                />

                <div>
                  <label class="block text-sm font-medium text-gray-300 mb-1">
                    Bio
                  </label>
                  <Textarea
                    value={bio()}
                    onInput={(e) => setBio(e.currentTarget.value)}
                    rows={5}
                    placeholder="Write something about yourself..."
                  />
                </div>

                <div class="flex justify-end">
                  <Button type="submit" variant="primary" disabled={saving()}>
                    {saving() ? 'Saving...' : 'Save Profile'}
                  </Button>
                </div>
              </form>

              {/* Password Change Form */}
              <form onSubmit={handleChangePassword} class="card space-y-4">
                <h2 class="text-lg font-medium text-neon-cyan">Change Password</h2>

                <Show when={passwordError()}>
                  <Alert type="error">{passwordError()}</Alert>
                </Show>
                <Show when={passwordSuccess()}>
                  <Alert type="success">{passwordSuccess()}</Alert>
                </Show>

                <Input
                  label="Current Password"
                  type="password"
                  value={currentPassword()}
                  onInput={(e) => setCurrentPassword(e.currentTarget.value)}
                  required
                />

                <Input
                  label="New Password"
                  type="password"
                  value={newPassword()}
                  onInput={(e) => setNewPassword(e.currentTarget.value)}
                  minLength={8}
                  maxLength={128}
                  required
                />

                <Input
                  label="Confirm New Password"
                  type="password"
                  value={confirmPassword()}
                  onInput={(e) => setConfirmPassword(e.currentTarget.value)}
                  required
                />

                <div class="flex justify-end">
                  <Button type="submit" variant="primary" disabled={saving()}>
                    {saving() ? 'Changing...' : 'Change Password'}
                  </Button>
                </div>
              </form>
            </div>
          )}
        </Show>
      </Show>
    </div>
  );
};

/**
 * User profile page - Shows another user's public profile.
 */
export const UserProfilePage: Component = () => {
  const params = useParams<{ username: string }>();
  const navigate = useNavigate();

  const [profile] = createResource(
    () => params.username,
    async (username) => {
      return userApi.getUserByUsername(username);
    }
  );

  const formatDate = (dateStr?: string) => {
    if (!dateStr) return '-';
    return new Date(dateStr).toLocaleString('ja-JP');
  };

  const getRoleLabel = (role: string) => {
    switch (role) {
      case 'sysop': return 'SysOp';
      case 'subop': return 'SubOp';
      case 'member': return 'Member';
      case 'guest': return 'Guest';
      default: return role;
    }
  };

  return (
    <div class="space-y-6">
      {/* Header */}
      <div class="flex items-center justify-between">
        <h1 class="text-2xl font-display font-bold text-neon-cyan">
          User Profile
        </h1>
        <Button variant="secondary" onClick={() => navigate(-1)}>
          Back
        </Button>
      </div>

      {/* Profile Content */}
      <Show
        when={!profile.loading}
        fallback={<PageLoading />}
      >
        <Show
          when={!profile.error}
          fallback={
            <Alert type="error">User not found</Alert>
          }
        >
          <Show when={profile()}>
            {(user) => (
              <div class="card space-y-6">
                {/* Basic Info */}
                <div class="space-y-4">
                  <ProfileField label="Username" value={user().username} />
                  <ProfileField label="Nickname" value={user().nickname} />
                  <ProfileField label="Role" value={getRoleLabel(user().role)} />
                  <ProfileField label="Registered" value={formatDate(user().created_at)} />
                  <ProfileField label="Last Login" value={formatDate(user().last_login_at)} />
                </div>

                {/* Profile / Bio */}
                <div class="border-t border-neon-cyan/20 pt-4">
                  <h3 class="text-sm font-medium text-gray-400 mb-2">Bio</h3>
                  <div class="text-gray-200 whitespace-pre-wrap">
                    {user().profile || <span class="text-gray-500 italic">No bio set</span>}
                  </div>
                </div>

                {/* Actions */}
                <div class="border-t border-neon-cyan/20 pt-4 flex space-x-4">
                  <Button
                    variant="primary"
                    onClick={() => navigate(`/mail?to=${encodeURIComponent(user().username)}`)}
                  >
                    Send Mail
                  </Button>
                </div>
              </div>
            )}
          </Show>
        </Show>
      </Show>
    </div>
  );
};

/**
 * Profile field component for displaying label-value pairs.
 */
const ProfileField: Component<{ label: string; value: string }> = (props) => {
  return (
    <div class="flex flex-col sm:flex-row sm:items-center">
      <span class="text-sm font-medium text-gray-400 sm:w-32">{props.label}</span>
      <span class="text-gray-200">{props.value}</span>
    </div>
  );
};
