import { type Component, createSignal, createEffect, onCleanup, For, Show } from 'solid-js';
import { Button, Input, Alert, Empty } from '../components';
import { getChatWebSocket } from '../api/chat';
import type { ChatRoom, ServerMessage, ChatParticipant } from '../types';
import { useI18n } from '../stores/i18n';

interface ChatMessage {
  type: 'chat' | 'action' | 'system' | 'join' | 'leave';
  username?: string;
  content: string;
  timestamp: string;
}

export const ChatPage: Component = () => {
  const { t } = useI18n();
  const [rooms, setRooms] = createSignal<ChatRoom[]>([]);
  const [currentRoom, setCurrentRoom] = createSignal<{ id: string; name: string } | null>(null);
  const [participants, setParticipants] = createSignal<ChatParticipant[]>([]);
  const [messages, setMessages] = createSignal<ChatMessage[]>([]);
  const [inputMessage, setInputMessage] = createSignal('');
  const [connected, setConnected] = createSignal(false);
  const [error, setError] = createSignal('');

  let messagesContainer: HTMLDivElement | undefined;
  const ws = getChatWebSocket();

  // WebSocket handlers - room list is received automatically on connect
  createEffect(() => {
    ws.onConnect(() => {
      setConnected(true);
      setError('');
    });

    ws.onDisconnect(() => {
      setConnected(false);
      setCurrentRoom(null);
      setParticipants([]);
    });

    ws.onError(() => {
      setError(t('chat.connectionError'));
    });

    ws.onMessage((message: ServerMessage) => {
      switch (message.type) {
        case 'joined':
          setCurrentRoom({ id: message.room_id, name: message.room_name });
          setParticipants(message.participants);
          setMessages([]);
          break;

        case 'left':
          setCurrentRoom(null);
          setParticipants([]);
          setMessages([]);
          break;

        case 'chat':
          addMessage({
            type: 'chat',
            username: message.username,
            content: message.content,
            timestamp: message.timestamp,
          });
          break;

        case 'action':
          addMessage({
            type: 'action',
            username: message.username,
            content: message.content,
            timestamp: message.timestamp,
          });
          break;

        case 'system':
          addMessage({
            type: 'system',
            content: message.content,
            timestamp: message.timestamp,
          });
          break;

        case 'user_joined':
          setParticipants((prev) => [...prev, { user_id: message.user_id, username: message.username }]);
          addMessage({
            type: 'join',
            username: message.username,
            content: `${message.username} が入室しました`,
            timestamp: message.timestamp,
          });
          break;

        case 'user_left':
          setParticipants((prev) => prev.filter((p) => p.user_id !== message.user_id));
          addMessage({
            type: 'leave',
            username: message.username,
            content: `${message.username} が退室しました`,
            timestamp: message.timestamp,
          });
          break;

        case 'room_list':
          setRooms(message.rooms);
          break;

        case 'error':
          setError(message.message);
          break;
      }
    });

    ws.connect();

    onCleanup(() => {
      ws.disconnect();
    });
  });

  const addMessage = (msg: ChatMessage) => {
    setMessages((prev) => [...prev, msg]);
    // Scroll to bottom
    setTimeout(() => {
      if (messagesContainer) {
        messagesContainer.scrollTop = messagesContainer.scrollHeight;
      }
    }, 0);
  };

  const handleJoinRoom = (roomId: string) => {
    ws.joinRoom(roomId);
  };

  const handleLeaveRoom = () => {
    ws.leaveRoom();
  };

  const handleSendMessage = (e: Event) => {
    e.preventDefault();
    const msg = inputMessage().trim();
    if (!msg) return;

    if (msg.startsWith('/me ')) {
      ws.sendAction(msg.slice(4));
    } else {
      ws.sendMessage(msg);
    }
    setInputMessage('');
  };

  return (
    <div class="space-y-6">
      <h1 class="text-2xl font-display font-bold text-neon-cyan">{t('chat.title')}</h1>

      <Show when={error()}>
        <Alert type="error" onClose={() => setError('')}>
          {error()}
        </Alert>
      </Show>

      <div class="grid grid-cols-1 lg:grid-cols-4 gap-6">
        {/* Room List */}
        <div class="lg:col-span-1">
          <div class="card">
            <h3 class="font-medium text-neon-cyan mb-4">{t('chat.roomList')}</h3>
            <div class="space-y-2">
              <Show
                when={rooms().length > 0}
                fallback={<p class="text-sm text-gray-500">{t('chat.noRooms')}</p>}
              >
                <For each={rooms()}>
                  {(room) => (
                    <button
                      onClick={() => handleJoinRoom(room.id)}
                      disabled={currentRoom()?.id === room.id}
                      class={`w-full text-left px-3 py-2 rounded transition-all duration-200 ${
                        currentRoom()?.id === room.id
                          ? 'bg-neon-cyan/20 text-neon-cyan'
                          : 'text-gray-400 hover:bg-neon-cyan/10 hover:text-gray-200'
                      }`}
                    >
                      <div class="font-medium">{room.name}</div>
                      <div class="text-xs text-gray-500">{room.participant_count} {t('chat.participants')}</div>
                    </button>
                  )}
                </For>
              </Show>
            </div>
          </div>
        </div>

        {/* Chat Area */}
        <div class="lg:col-span-3">
          <Show
            when={currentRoom()}
            fallback={
              <div class="card">
                <Empty
                  title={t('chat.selectRoom')}
                  description={t('chat.selectRoomDesc')}
                />
              </div>
            }
          >
            <div class="card flex flex-col h-[600px]">
              {/* Room Header */}
              <div class="flex items-center justify-between pb-4 border-b border-neon-cyan/20">
                <div>
                  <h3 class="font-medium text-neon-cyan">{currentRoom()!.name}</h3>
                  <p class="text-xs text-gray-500">{participants().length} {t('chat.participantsOnline')}</p>
                </div>
                <Button variant="secondary" onClick={handleLeaveRoom}>
                  {t('chat.leave')}
                </Button>
              </div>

              {/* Messages */}
              <div
                ref={messagesContainer}
                class="flex-1 overflow-y-auto py-4 space-y-2"
              >
                <For each={messages()}>
                  {(msg) => <ChatMessageItem message={msg} />}
                </For>
              </div>

              {/* Input */}
              <form onSubmit={handleSendMessage} class="pt-4 border-t border-neon-cyan/20">
                <div class="flex space-x-2">
                  <Input
                    value={inputMessage()}
                    onInput={(e) => setInputMessage(e.currentTarget.value)}
                    placeholder={t('chat.inputPlaceholder')}
                    class="flex-1"
                  />
                  <Button type="submit" variant="primary">
                    {t('common.send')}
                  </Button>
                </div>
              </form>
            </div>
          </Show>
        </div>
      </div>

      {/* Connection Status */}
      <div class="text-xs text-gray-600 text-center">
        <span class={`inline-block w-2 h-2 rounded-full mr-2 ${connected() ? 'bg-neon-green' : 'bg-neon-pink'}`} />
        {connected() ? t('chat.connected') : t('chat.disconnected')}
      </div>
    </div>
  );
};

interface ChatMessageItemProps {
  message: ChatMessage;
}

const ChatMessageItem: Component<ChatMessageItemProps> = (props) => {
  const msg = props.message;
  const time = new Date(msg.timestamp).toLocaleTimeString('ja-JP', {
    hour: '2-digit',
    minute: '2-digit',
  });

  if (msg.type === 'system' || msg.type === 'join' || msg.type === 'leave') {
    return (
      <div class="text-center text-xs text-gray-500 py-1">
        <span class="text-gray-600">[{time}]</span> {msg.content}
      </div>
    );
  }

  if (msg.type === 'action') {
    return (
      <div class="text-sm text-neon-purple italic px-2 py-1">
        <span class="text-gray-600 text-xs">[{time}]</span>{' '}
        * {msg.username} {msg.content}
      </div>
    );
  }

  return (
    <div class="px-2 py-1">
      <span class="text-gray-600 text-xs">[{time}]</span>{' '}
      <span class="text-neon-cyan font-medium">{msg.username}:</span>{' '}
      <span class="text-gray-300">{msg.content}</span>
    </div>
  );
};
