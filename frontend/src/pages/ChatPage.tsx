import { useEffect, useRef, useState } from "react";
import ReactMarkdown from "react-markdown";
import {
  useConversation,
  useConversations,
  useCreateConversation,
  useDeleteConversation,
  useSendMessage,
} from "../api/chat";
import type { ChatResponse, Message } from "../api/types";
import CitationList from "../components/CitationList";

function initials(role: string) {
  return role === "user" ? "You" : "AI";
}

function formatTime(iso: string) {
  return new Date(iso).toLocaleTimeString([], { hour: "numeric", minute: "2-digit" });
}

export default function ChatPage() {
  const conversations = useConversations();
  const [selected, setSelected] = useState<string | null>(null);
  const conversation = useConversation(selected ?? undefined);
  const createConversation = useCreateConversation();
  const deleteConversation = useDeleteConversation();
  const sendMessage = useSendMessage();
  const [draft, setDraft] = useState("");
  const [pending, setPending] = useState<Message | null>(null);
  const [lastResponse, setLastResponse] = useState<ChatResponse | null>(null);
  const scrollRef = useRef<HTMLDivElement>(null);

  const messages: Message[] = conversation.data?.messages ?? [];
  const allMessages = pending ? [...messages, pending] : messages;

  useEffect(() => {
    scrollRef.current?.scrollTo({ top: scrollRef.current.scrollHeight, behavior: "smooth" });
  }, [allMessages.length, sendMessage.isPending]);

  const handleNewChat = () => {
    createConversation.mutate(undefined, {
      onSuccess: (conversation) => setSelected(conversation.id),
    });
  };

  const handleSend = async () => {
    const content = draft.trim();
    if (!content || !selected) return;
    setDraft("");
    setLastResponse(null);
    setPending({
      id: "pending",
      conversation_id: selected,
      role: "user",
      content,
      tool_calls: null,
      tool_results: null,
      citations: null,
      created_at: new Date().toISOString(),
    });
    try {
      const response = await sendMessage.mutateAsync({ id: selected, content });
      setLastResponse(response);
    } catch {
      // error surfaces via sendMessage.error
    } finally {
      setPending(null);
    }
  };

  return (
    <div className="chat-layout">
      <div className="card conversation-list">
        <div className="page-header" style={{ marginBottom: 8 }}>
          <h3>Chats</h3>
          <button className="primary" onClick={handleNewChat}>
            New
          </button>
        </div>
        <div className="conv-scroll">
          {conversations.data?.map((conv) => (
            <button
              key={conv.id}
              className={selected === conv.id ? "active" : ""}
              onClick={() => {
                setSelected(conv.id);
                setLastResponse(null);
              }}
            >
              <span className="conv-title">{conv.title}</span>
              <span
                className="conv-close"
                onClick={(e) => {
                  e.stopPropagation();
                  if (confirm("Delete this chat?")) {
                    deleteConversation.mutate(conv.id);
                    if (selected === conv.id) setSelected(null);
                  }
                }}
              >
                ×
              </span>
            </button>
          ))}
          {conversations.data && conversations.data.length === 0 && (
            <div className="empty">No chats yet.</div>
          )}
        </div>
      </div>

      <div className="card chat-panel">
        {!selected ? (
          <div className="empty" style={{ padding: "1.25rem" }}>
            Pick a chat on the left or start a new one. The assistant can manage tasks, calendar
            events and answer questions about uploaded documents (with citations).
          </div>
        ) : (
          <>
            <div className="chat-panel-header">
              <h3>{conversations.data?.find((c) => c.id === selected)?.title ?? "Chat"}</h3>
            </div>

            <div className="chat-messages" ref={scrollRef}>
              {allMessages.length === 0 && (
                <div className="empty">Say hello and ask for help, e.g. “what's due today?”</div>
              )}
              {allMessages.map((msg) => {
                if (msg.role === "tool") return null;
                return (
                  <div className={`message-row ${msg.role}`} key={msg.id}>
                    <div className={`avatar ${msg.role}`}>{initials(msg.role)}</div>
                    <div className="message-col">
                      <div className={`bubble ${msg.role}`}>
                        {msg.role === "assistant" ? (
                          <div className="markdown">
                            <ReactMarkdown>{msg.content}</ReactMarkdown>
                          </div>
                        ) : (
                          msg.content
                        )}
                      </div>
                      {msg.role === "assistant" && msg.citations && (
                        <CitationList citations={msg.citations} />
                      )}
                      <span className="message-time">{formatTime(msg.created_at)}</span>
                    </div>
                  </div>
                );
              })}
              {sendMessage.isPending && (
                <div className="message-row assistant thinking-row">
                  <div className="avatar assistant">AI</div>
                  <div className="thinking-dots">
                    <span />
                    <span />
                    <span />
                  </div>
                </div>
              )}
            </div>

            {lastResponse && lastResponse.tool_calls.length > 0 && (
              <details className="trace">
                <summary>{lastResponse.tool_calls.length} tool call(s) ran</summary>
                {lastResponse.tool_calls.map((trace, index) => (
                  <div className="trace-call" key={index}>
                    <strong>{trace.name}</strong>
                    <pre>{JSON.stringify(trace.args, null, 2)}</pre>
                    <pre>{JSON.stringify(trace.result, null, 2)}</pre>
                  </div>
                ))}
              </details>
            )}

            {sendMessage.error && (
              <div className="error" style={{ margin: "0 1.1rem 0.6rem" }}>
                {(sendMessage.error as Error).message}
              </div>
            )}

            <div className="chat-input">
              <input
                placeholder="Message the assistant…"
                value={draft}
                onChange={(e) => setDraft(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter" && !e.shiftKey) {
                    e.preventDefault();
                    handleSend();
                  }
                }}
              />
              <button
                className="primary"
                onClick={handleSend}
                disabled={!draft.trim() || !selected}
                aria-label="Send message"
                title="Send"
              >
                ↑
              </button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
