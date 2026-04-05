const state = {
  activeSessionId: null,
  sessions: [],
  auth: null,
  health: null,
  loading: false,
  currentSession: null,
  liveTurn: null,
};

const elements = {
  authPill: document.querySelector("#auth-pill"),
  authSummary: document.querySelector("#auth-summary"),
  authMeta: document.querySelector("#auth-meta"),
  loginButton: document.querySelector("#login-button"),
  logoutButton: document.querySelector("#logout-button"),
  healthPill: document.querySelector("#health-pill"),
  runtimeMeta: document.querySelector("#runtime-meta"),
  workspaceTitle: document.querySelector("#workspace-title"),
  workspaceCopy: document.querySelector("#workspace-copy"),
  requestStatus: document.querySelector("#request-status"),
  sessionList: document.querySelector("#session-list"),
  sessionTitle: document.querySelector("#session-title"),
  messageList: document.querySelector("#message-list"),
  flash: document.querySelector("#flash"),
  chatForm: document.querySelector("#chat-form"),
  promptInput: document.querySelector("#prompt-input"),
  modelInput: document.querySelector("#model-input"),
  permissionSelect: document.querySelector("#permission-select"),
  allowedToolsInput: document.querySelector("#allowed-tools-input"),
  enableToolsInput: document.querySelector("#enable-tools-input"),
  sendButton: document.querySelector("#send-button"),
  refreshSessionsButton: document.querySelector("#refresh-sessions-button"),
  newSessionButton: document.querySelector("#new-session-button"),
  compactButton: document.querySelector("#compact-button"),
  composerHint: document.querySelector("#composer-hint"),
};

async function api(path, options = {}) {
  const response = await fetch(path, {
    headers: {
      "content-type": "application/json",
      ...(options.headers || {}),
    },
    ...options,
  });
  const contentType = response.headers.get("content-type") || "";
  const payload = contentType.includes("application/json")
    ? await response.json()
    : await response.text();

  if (!response.ok) {
    const message =
      typeof payload === "string"
        ? payload
        : payload && typeof payload.error === "string"
          ? payload.error
          : `Request failed with status ${response.status}`;
    throw new Error(message);
  }

  return payload;
}

function setFlash(message, tone = "info") {
  if (!message) {
    elements.flash.hidden = true;
    elements.flash.textContent = "";
    return;
  }
  elements.flash.hidden = false;
  elements.flash.dataset.tone = tone;
  elements.flash.textContent = message;
}

function setBusy(isBusy, label = "Working") {
  state.loading = isBusy;
  elements.sendButton.disabled = isBusy || Boolean(state.auth && !state.auth.inference_ready);
  elements.refreshSessionsButton.disabled = isBusy;
  elements.compactButton.disabled = isBusy || !state.activeSessionId;
  elements.requestStatus.textContent = isBusy ? label : "Ready";
}

function pillClass(tone) {
  if (tone === "good") return "pill";
  if (tone === "warn") return "pill pill--warn";
  if (tone === "bad") return "pill pill--bad";
  return "pill pill--muted";
}

function updateAuthPanel(auth) {
  state.auth = auth;

  let tone = "muted";
  let label = "No auth";
  if (auth.warning) {
    tone = "warn";
    label = auth.active_source === "oauth" ? "OAuth saved" : "Auth warning";
  } else if (auth.active_source === "oauth" && auth.saved_oauth_expired) {
    tone = "warn";
    label = "OAuth expired";
  } else if (auth.inference_ready) {
    tone = "good";
    label = auth.active_source === "oauth" ? "OAuth active" : "Env auth";
  } else if (auth.authenticated) {
    tone = "warn";
    label = "Auth incomplete";
  }

  elements.authPill.className = pillClass(tone);
  elements.authPill.textContent = label;

  const summaryParts = [];
  if (auth.warning) {
    summaryParts.push(auth.warning);
  } else if (auth.active_source === "oauth") {
    summaryParts.push("Requests will use saved Claude OAuth credentials.");
  } else if (auth.active_source === "api_key" || auth.active_source === "api_key_and_bearer") {
    summaryParts.push("Environment credentials override saved OAuth for API requests.");
  } else if (auth.active_source === "bearer") {
    summaryParts.push("A bearer token from the environment is active.");
  } else {
    summaryParts.push("No active credentials detected yet.");
  }
  if (auth.saved_oauth_expired) {
    summaryParts.push("The saved OAuth token is expired and may need a fresh login.");
  }
  elements.authSummary.textContent = summaryParts.join(" ");

  const meta = [
    ["Active source", auth.active_source],
    ["Inference ready", auth.inference_ready ? "yes" : "no"],
    ["Saved OAuth", auth.saved_oauth ? "yes" : "no"],
    ["OAuth expired", auth.saved_oauth_expired ? "yes" : "no"],
    ["Scopes", auth.scopes.length ? auth.scopes.join(", ") : "none"],
    ["Credentials path", auth.credentials_path || "unavailable"],
  ];
  elements.authMeta.innerHTML = meta
    .map(
      ([term, value]) =>
        `<div class="meta-row"><dt>${escapeHtml(term)}</dt><dd>${escapeHtml(String(value))}</dd></div>`,
    )
    .join("");
  elements.sendButton.disabled = state.loading || !auth.inference_ready;
}

function updateHealthPanel(health) {
  state.health = health;
  elements.healthPill.className = pillClass("good");
  elements.healthPill.textContent = `v${health.version}`;
  elements.workspaceTitle.textContent = health.workspace_root;
  elements.workspaceCopy.textContent = `Runtime date context: ${health.date}. Sessions and config resolve from this workspace root.`;
  elements.runtimeMeta.innerHTML = [
    ["Workspace root", health.workspace_root],
    ["Runtime version", health.version],
    ["Date context", health.date],
  ]
    .map(
      ([term, value]) =>
        `<div class="meta-row"><dt>${escapeHtml(term)}</dt><dd>${escapeHtml(String(value))}</dd></div>`,
    )
    .join("");
}

function renderSessions() {
  if (!state.sessions.length) {
    elements.sessionList.innerHTML =
      '<article class="session-card"><div class="session-card__title">No saved sessions yet</div><div class="session-card__meta">The first prompt will create one under .claw/sessions/.</div></article>';
    return;
  }

  elements.sessionList.innerHTML = state.sessions
    .map((session) => {
      const isActive = session.id === state.activeSessionId;
      const modified = new Date(Number(session.modified_epoch_millis)).toLocaleString();
      return `
        <button class="session-card ${isActive ? "is-active" : ""}" type="button" data-session-id="${escapeHtml(session.id)}">
          <div class="session-card__title">${escapeHtml(session.id)}</div>
          <div class="session-card__meta">${escapeHtml(String(session.message_count))} messages</div>
          <div class="session-card__meta">${escapeHtml(modified)}</div>
        </button>
      `;
    })
    .join("");

  for (const button of elements.sessionList.querySelectorAll("[data-session-id]")) {
    button.addEventListener("click", () => {
      loadSession(button.dataset.sessionId).catch(handleError);
    });
  }
}

function renderSession(session) {
  state.currentSession = session;
  state.liveTurn = null;
  state.activeSessionId = session.id;
  elements.sessionTitle.textContent = session.id;
  elements.composerHint.innerHTML = `Current session: <code>${escapeHtml(
    session.id,
  )}</code> with ${escapeHtml(String(session.message_count))} saved messages.`;
  elements.compactButton.disabled = state.loading ? true : false;
  renderVisibleMessages();
}

function renderVisibleMessages() {
  const messages = visibleMessages();
  if (!messages.length) {
    elements.messageList.innerHTML =
      '<article class="message message--empty"><p>No messages yet. Start with a prompt or load an existing session.</p></article>';
    return;
  }

  elements.messageList.innerHTML = messages
    .map((message) => {
      const role = message.role;
      const usage = message.usage
        ? `${message.usage.total_tokens} tokens · ${message.usage.estimated_cost_usd}`
        : "No token record";
      return `
        <article class="message message--${escapeHtml(role)}">
          <div class="message__header">
            <div class="message__role">${escapeHtml(role)}</div>
            <div class="message__usage">${escapeHtml(usage)}</div>
          </div>
          <div class="message__blocks">
            ${message.blocks.map(renderBlock).join("")}
          </div>
        </article>
      `;
    })
    .join("");

  elements.messageList.scrollTop = elements.messageList.scrollHeight;
}

function visibleMessages() {
  const messages = state.currentSession?.messages
    ? [...state.currentSession.messages]
    : [];

  if (!state.liveTurn) {
    return messages;
  }

  messages.push({
    role: "user",
    blocks: [{ type: "text", text: state.liveTurn.prompt }],
    usage: null,
  });

  const assistantBlocks = [];
  if (state.liveTurn.assistantText) {
    assistantBlocks.push({
      type: "text",
      text: state.liveTurn.assistantText,
    });
  }
  assistantBlocks.push(...state.liveTurn.blocks);

  if (!assistantBlocks.length) {
    assistantBlocks.push({
      type: "text",
      text: "Thinking...",
    });
  }

  messages.push({
    role: "assistant",
    blocks: assistantBlocks,
    usage: state.liveTurn.usage,
  });

  return messages;
}

function renderBlock(block) {
  if (block.type === "text") {
    return `
      <section class="block">
        <pre>${escapeHtml(block.text)}</pre>
      </section>
    `;
  }

  if (block.type === "tool_use") {
    return `
      <section class="block block--tool-use">
        <div class="block__label">Tool Use</div>
        <div class="block__meta">${escapeHtml(block.name)} · ${escapeHtml(block.id)}</div>
        <pre>${escapeHtml(block.input)}</pre>
      </section>
    `;
  }

  if (block.type === "tool_activity") {
    return `
      <section class="block ${block.is_error ? "block--tool-result" : "block--tool-use"}">
        <div class="block__label">${escapeHtml(block.label)}</div>
        <div class="block__meta">${escapeHtml(block.tool_name)}</div>
        <pre>${escapeHtml(block.output)}</pre>
      </section>
    `;
  }

  return `
    <section class="block block--tool-result">
      <div class="block__label">${block.is_error ? "Tool Error" : "Tool Result"}</div>
      <div class="block__meta">${escapeHtml(block.tool_name)} · ${escapeHtml(
        block.tool_use_id || "live",
      )}</div>
      <pre>${escapeHtml(block.output)}</pre>
    </section>
  `;
}

function startFreshSession() {
  state.activeSessionId = null;
  state.currentSession = null;
  state.liveTurn = null;
  elements.sessionTitle.textContent = "New session";
  elements.composerHint.innerHTML =
    'The next prompt will create a new saved session under <code>.claw/sessions/</code>.';
  elements.compactButton.disabled = true;
  renderVisibleMessages();
  renderSessions();
}

async function refreshAuthStatus() {
  const auth = await api("/api/auth/status");
  updateAuthPanel(auth);
}

async function refreshHealth() {
  const health = await api("/api/health");
  updateHealthPanel(health);
}

async function refreshSessions({ preferLatest = false } = {}) {
  state.sessions = await api("/api/sessions");
  renderSessions();
  if (preferLatest && !state.activeSessionId && state.sessions.length) {
    await loadSession(state.sessions[0].id);
  }
}

async function loadSession(sessionId) {
  const session = await api(`/api/sessions/${encodeURIComponent(sessionId)}`);
  renderSession(session);
  renderSessions();
  setFlash(`Loaded session ${session.id}.`, "info");
}

async function sendPrompt(event) {
  event.preventDefault();
  if (state.auth && !state.auth.inference_ready) {
    setFlash(
      state.auth.warning ||
        "This runtime is not ready to send prompts yet. Configure ANTHROPIC_API_KEY or a supported auth source.",
      "bad",
    );
    return;
  }
  const prompt = elements.promptInput.value.trim();
  if (!prompt) {
    setFlash("Enter a prompt before sending.", "bad");
    return;
  }

  const payload = {
    prompt,
    session_id: state.activeSessionId,
    model: elements.modelInput.value.trim() || undefined,
    permission_mode: elements.permissionSelect.value,
    allowed_tools: parseAllowedTools(elements.allowedToolsInput.value),
    enable_tools: elements.enableToolsInput.checked,
  };

  setBusy(true, "Streaming turn");
  setFlash("", "info");
  beginLiveTurn(payload);

  try {
    await streamJsonEvents("/api/chat/stream", payload, async ({ event: name, data }) => {
      await handleStreamEvent(name, data);
    });
    if (state.loading) {
      throw new Error("The stream closed before the turn reported completion.");
    }
  } catch (error) {
    if (state.liveTurn) {
      pushLiveBlock({
        type: "tool_activity",
        label: "Runtime Error",
        tool_name: "runtime",
        output: error.message || String(error),
        is_error: true,
      });
    }
    throw error;
  } finally {
    if (state.loading) {
      setBusy(false);
    }
  }
}

function beginLiveTurn(payload) {
  state.liveTurn = {
    prompt: payload.prompt,
    assistantText: "",
    blocks: [],
    usage: null,
    sessionId: payload.session_id || null,
  };
  renderVisibleMessages();
}

function pushLiveBlock(block) {
  if (!state.liveTurn) {
    return;
  }
  state.liveTurn.blocks.push(block);
  renderVisibleMessages();
}

async function handleStreamEvent(name, data) {
  if (name === "turn_started") {
    if (state.liveTurn) {
      state.liveTurn.sessionId = data.session_id;
    }
    state.activeSessionId = data.session_id;
    elements.sessionTitle.textContent = data.session_id;
    elements.composerHint.innerHTML = `Streaming into session <code>${escapeHtml(
      data.session_id,
    )}</code> with model <code>${escapeHtml(data.model)}</code>.`;
    elements.requestStatus.textContent = `Streaming ${data.model}`;
    renderSessions();
    return;
  }

  if (name === "assistant_text_delta") {
    if (state.liveTurn) {
      state.liveTurn.assistantText += data.text;
      renderVisibleMessages();
    }
    return;
  }

  if (name === "assistant_tool_use") {
    pushLiveBlock({
      type: "tool_use",
      id: data.id,
      name: data.name,
      input: data.input,
    });
    return;
  }

  if (name === "assistant_usage") {
    if (state.liveTurn) {
      state.liveTurn.usage = data;
      renderVisibleMessages();
    }
    elements.requestStatus.textContent = `Streaming · ${data.total_tokens} tokens`;
    return;
  }

  if (name === "tool_execution_started") {
    pushLiveBlock({
      type: "tool_activity",
      label: "Tool Start",
      tool_name: data.tool_name,
      output: data.input,
      is_error: false,
    });
    return;
  }

  if (name === "tool_execution_finished") {
    pushLiveBlock({
      type: "tool_result",
      tool_use_id: "live",
      tool_name: data.tool_name,
      output: data.output,
      is_error: data.is_error,
    });
    return;
  }

  if (name === "prompt_cache") {
    elements.requestStatus.textContent = data.unexpected
      ? "Prompt cache changed"
      : "Prompt cache hit";
    return;
  }

  if (name === "done") {
    elements.promptInput.value = "";
    renderSession(data.session);
    await refreshSessions();
    setFlash(
      `Turn complete. ${data.usage.total_tokens} tokens, ${data.usage.estimated_cost_usd}.`,
      "good",
    );
    setBusy(false);
    return;
  }

  if (name === "error") {
    setFlash(data.message || "Streaming request failed.", "bad");
    setBusy(false);
    throw new Error(data.message || "Streaming request failed.");
  }
}

async function streamJsonEvents(path, payload, onEvent) {
  const response = await fetch(path, {
    method: "POST",
    headers: {
      "content-type": "application/json",
      accept: "text/event-stream",
    },
    body: JSON.stringify(payload),
  });

  if (!response.ok) {
    const contentType = response.headers.get("content-type") || "";
    const body = contentType.includes("application/json")
      ? await response.json()
      : await response.text();
    const message =
      typeof body === "string"
        ? body
        : body && typeof body.error === "string"
          ? body.error
          : `Request failed with status ${response.status}`;
    throw new Error(message);
  }

  if (!response.body) {
    throw new Error("The browser did not expose a readable streaming body.");
  }

  const reader = response.body.getReader();
  const decoder = new TextDecoder();
  let buffer = "";

  while (true) {
    const { value, done } = await reader.read();
    buffer += decoder.decode(value || new Uint8Array(), { stream: !done });
    const consumed = consumeSseBuffer(buffer);
    buffer = consumed.buffer;
    for (const event of consumed.events) {
      await onEvent(event);
    }
    if (done) {
      break;
    }
  }

  if (buffer.trim()) {
    const consumed = consumeSseBuffer(`${buffer}\n\n`);
    for (const event of consumed.events) {
      await onEvent(event);
    }
  }
}

function consumeSseBuffer(buffer) {
  const normalized = buffer.replaceAll("\r\n", "\n");
  const events = [];
  let rest = normalized;
  let boundary = rest.indexOf("\n\n");

  while (boundary >= 0) {
    const rawEvent = rest.slice(0, boundary);
    rest = rest.slice(boundary + 2);
    const parsed = parseSseEvent(rawEvent);
    if (parsed) {
      events.push(parsed);
    }
    boundary = rest.indexOf("\n\n");
  }

  return { buffer: rest, events };
}

function parseSseEvent(rawEvent) {
  const lines = rawEvent.split("\n");
  let name = "message";
  const dataLines = [];

  for (const line of lines) {
    if (!line || line.startsWith(":")) {
      continue;
    }
    if (line.startsWith("event:")) {
      name = line.slice(6).trim();
      continue;
    }
    if (line.startsWith("data:")) {
      dataLines.push(line.slice(5).trimStart());
    }
  }

  if (!dataLines.length) {
    return null;
  }

  const rawData = dataLines.join("\n");
  try {
    return {
      event: name,
      data: JSON.parse(rawData),
    };
  } catch {
    return {
      event: name,
      data: { message: rawData },
    };
  }
}

function parseAllowedTools(raw) {
  return raw
    .split(/[\s,]+/)
    .map((value) => value.trim())
    .filter(Boolean);
}

function hasFreshSavedOauth(auth) {
  return Boolean(auth && auth.saved_oauth && !auth.saved_oauth_expired);
}

function sleep(ms) {
  return new Promise((resolve) => {
    window.setTimeout(resolve, ms);
  });
}

async function waitForOauthCompletion({ popup, redirectUri, timeoutMs = 120000 }) {
  const startedAt = Date.now();
  while (Date.now() - startedAt < timeoutMs) {
    await sleep(1500);
    const auth = await api("/api/auth/status");
    updateAuthPanel(auth);
    if (hasFreshSavedOauth(auth)) {
      if (popup && !popup.closed) {
        popup.close();
      }
      return auth;
    }
  }

  throw new Error(
    `Claude OAuth did not complete in time. The browser must return to ${redirectUri}. If claw-web runs in Docker, publish -p 4545:4545.`,
  );
}

async function runLogin() {
  const response = await api("/api/auth/login/start", { method: "POST" });
  const popup = window.open(
    response.authorize_url,
    "claw-oauth",
    "popup=yes,width=720,height=840,resizable=yes,scrollbars=yes",
  );
  if (!popup) {
    throw new Error("The browser blocked the OAuth popup. Allow popups and try again.");
  }
  setFlash(
    `Claude OAuth window opened. Finish login in the popup and let the browser return to ${response.redirect_uri}. If claw-web runs in Docker, publish -p 4545:4545.`,
    "info",
  );
  const auth = await waitForOauthCompletion({
    popup,
    redirectUri: response.redirect_uri,
  });
  setFlash(
    auth.warning ||
      (auth.active_source === "oauth"
        ? "Claude OAuth login completed."
        : "Saved OAuth credentials were updated. Environment credentials still take precedence for API requests."),
    "good",
  );
}

async function runLogout() {
  await api("/api/auth/logout", { method: "POST" });
  await refreshAuthStatus();
  setFlash("Saved OAuth credentials were cleared.", "good");
}

async function compactCurrentSession() {
  if (!state.activeSessionId) {
    setFlash("Load or create a session first.", "bad");
    return;
  }
  setBusy(true, "Compacting");
  try {
    const session = await api(
      `/api/sessions/${encodeURIComponent(state.activeSessionId)}/compact`,
      { method: "POST" },
    );
    renderSession(session);
    await refreshSessions();
    setFlash("Session compaction completed.", "good");
  } finally {
    setBusy(false);
  }
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;");
}

function handleError(error) {
  console.error(error);
  setFlash(error.message || String(error), "bad");
  setBusy(false);
}

async function bootstrap() {
  setBusy(false);
  try {
    await refreshHealth();
    await refreshAuthStatus();
    await refreshSessions({ preferLatest: true });
    startFreshSession();
    if (state.sessions.length) {
      await loadSession(state.sessions[0].id);
    }
  } catch (error) {
    handleError(error);
  }
}

elements.chatForm.addEventListener("submit", (event) => {
  sendPrompt(event).catch(handleError);
});
elements.loginButton.addEventListener("click", () => {
  runLogin().catch(handleError);
});
elements.logoutButton.addEventListener("click", () => {
  runLogout().catch(handleError);
});
elements.refreshSessionsButton.addEventListener("click", () => {
  refreshSessions().catch(handleError);
});
elements.newSessionButton.addEventListener("click", () => {
  setFlash("Switched to a fresh unsaved session draft.", "info");
  startFreshSession();
});
elements.compactButton.addEventListener("click", () => {
  compactCurrentSession().catch(handleError);
});

window.addEventListener("message", (event) => {
  if (!event.data || event.data.type !== "claw-auth-complete") {
    return;
  }
  if (event.data.ok) {
    refreshAuthStatus().catch(handleError);
  } else {
    setFlash(event.data.message || "OAuth login failed.", "bad");
  }
});

bootstrap();
