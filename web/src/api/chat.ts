import { api } from './client';
import { getOneTimeToken } from './auth';
import type { ChatRoom, ClientMessage, ServerMessage } from '../types';

export async function getRooms(): Promise<ChatRoom[]> {
  return api.get<ChatRoom[]>('/chat/rooms');
}

export class ChatWebSocket {
  private ws: WebSocket | null = null;
  private reconnectAttempts = 0;
  private maxReconnectAttempts = 5;
  private reconnectDelay = 1000;
  private pingInterval: number | null = null;
  private onMessageHandler: ((message: ServerMessage) => void) | null = null;
  private onConnectHandler: (() => void) | null = null;
  private onDisconnectHandler: (() => void) | null = null;
  private onErrorHandler: ((error: Event) => void) | null = null;

  async connect(): Promise<void> {
    if (this.ws?.readyState === WebSocket.OPEN) {
      return;
    }

    try {
      // Get one-time token for WebSocket connection
      const tokenResponse = await getOneTimeToken('websocket');
      const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
      const host = window.location.host;
      const url = `${protocol}//${host}/api/chat/ws?token=${encodeURIComponent(tokenResponse.token)}`;

      this.ws = new WebSocket(url);
    } catch (error) {
      console.error('Failed to get one-time token for WebSocket:', error);
      this.onErrorHandler?.(new Event('token_error'));
      return;
    }

    this.ws.onopen = () => {
      this.reconnectAttempts = 0;
      this.startPing();
      this.onConnectHandler?.();
    };

    this.ws.onmessage = (event) => {
      try {
        const message = JSON.parse(event.data) as ServerMessage;
        this.onMessageHandler?.(message);
      } catch (e) {
        console.error('Failed to parse WebSocket message:', e);
      }
    };

    this.ws.onclose = () => {
      this.stopPing();
      this.onDisconnectHandler?.();
      this.attemptReconnect();
    };

    this.ws.onerror = (error) => {
      this.onErrorHandler?.(error);
    };
  }

  disconnect(): void {
    this.maxReconnectAttempts = 0; // Prevent reconnection
    this.stopPing();
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
  }

  send(message: ClientMessage): void {
    if (this.ws?.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(message));
    }
  }

  joinRoom(roomId: string): void {
    this.send({ type: 'join', room_id: roomId });
  }

  leaveRoom(): void {
    this.send({ type: 'leave' });
  }

  sendMessage(content: string): void {
    this.send({ type: 'message', content });
  }

  sendAction(content: string): void {
    this.send({ type: 'action', content });
  }

  onMessage(handler: (message: ServerMessage) => void): void {
    this.onMessageHandler = handler;
  }

  onConnect(handler: () => void): void {
    this.onConnectHandler = handler;
  }

  onDisconnect(handler: () => void): void {
    this.onDisconnectHandler = handler;
  }

  onError(handler: (error: Event) => void): void {
    this.onErrorHandler = handler;
  }

  isConnected(): boolean {
    return this.ws?.readyState === WebSocket.OPEN;
  }

  private startPing(): void {
    this.pingInterval = window.setInterval(() => {
      this.send({ type: 'ping' });
    }, 30000);
  }

  private stopPing(): void {
    if (this.pingInterval !== null) {
      clearInterval(this.pingInterval);
      this.pingInterval = null;
    }
  }

  private attemptReconnect(): void {
    if (this.reconnectAttempts >= this.maxReconnectAttempts) {
      return;
    }

    this.reconnectAttempts++;
    const delay = this.reconnectDelay * Math.pow(2, this.reconnectAttempts - 1);

    setTimeout(() => {
      this.connect().catch((error) => {
        console.error('Reconnect failed:', error);
      });
    }, delay);
  }
}

// Singleton instance
let chatWebSocket: ChatWebSocket | null = null;

export function getChatWebSocket(): ChatWebSocket {
  if (!chatWebSocket) {
    chatWebSocket = new ChatWebSocket();
  }
  return chatWebSocket;
}
