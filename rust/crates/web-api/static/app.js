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
  assistantPane: document.querySelector("#assistant-pane"),
  assistantPaneMeta: document.querySelector("#assistant-pane-meta"),
  flash: document.querySelector("#flash"),
  chatForm: document.querySelector("#chat-form"),
  promptInput: document.querySelector("#prompt-input"),
  modelInput: document.querySelector("#model-input"),
  providerSelect: document.querySelector("#provider-select"),
  systemPromptInput: document.querySelector("#system-prompt-input"),
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
  elements.sendButton.disabled =
    isBusy || Boolean(state.auth && !selectedProviderStatus(state.auth).ready);
  elements.refreshSessionsButton.disabled = isBusy;
  elements.compactButton.disabled = isBusy || !state.activeSessionId;
  elements.requestStatus.textContent = isBusy ? label : "Ready";
}

function autosizePromptInput() {
  const textarea = elements.promptInput;
  const styles = window.getComputedStyle(textarea);
  const lineHeight = Number.parseFloat(styles.lineHeight) || 24;
  const padding =
    (Number.parseFloat(styles.paddingTop) || 0) + (Number.parseFloat(styles.paddingBottom) || 0);
  const border =
    (Number.parseFloat(styles.borderTopWidth) || 0) +
    (Number.parseFloat(styles.borderBottomWidth) || 0);
  const minHeight = lineHeight + padding + border;
  const maxHeight = lineHeight * 3 + padding + border;

  textarea.style.height = "auto";
  const nextHeight = Math.min(maxHeight, Math.max(minHeight, textarea.scrollHeight));
  textarea.style.height = `${nextHeight}px`;
  textarea.style.overflowY = textarea.scrollHeight > maxHeight ? "auto" : "hidden";
}

function selectedProvider() {
  return elements.providerSelect.value || "auto";
}

function normalizedModel() {
  return (elements.modelInput.value || "").trim().toLowerCase();
}

function inferAutoProvider(auth, model) {
  if (model.startsWith("claude")) {
    return "anthropic";
  }
  if (model.startsWith("grok")) {
    return "xai";
  }
  if (model.startsWith("gemini")) {
    return "gemini";
  }
  if (model.startsWith("deepseek")) {
    return "deepseek";
  }
  if (model.startsWith("sonar")) {
    return "perplexity";
  }
  if (auth?.env_api_key || auth?.env_bearer_token || auth?.saved_oauth) {
    return "anthropic";
  }
  if (auth?.env_openai_api_key) {
    return "openai-compatible";
  }
  if (auth?.env_xai_api_key) {
    return "xai";
  }
  if (auth?.env_gemini_api_key) {
    return "gemini";
  }
  if (auth?.env_deepseek_api_key) {
    return "deepseek";
  }
  if (auth?.env_perplexity_api_key) {
    return "perplexity";
  }
  return "anthropic";
}

function effectiveProvider(auth, provider = selectedProvider(), model = normalizedModel()) {
  return provider === "auto" ? inferAutoProvider(auth, model) : provider;
}

function providerDisplayName(provider) {
  if (provider === "openai-compatible") {
    return "OpenAI-compatible";
  }
  if (provider === "xai") {
    return "xAI";
  }
  if (provider === "gemini") {
    return "Gemini";
  }
  if (provider === "deepseek") {
    return "DeepSeek";
  }
  if (provider === "perplexity") {
    return "Perplexity";
  }
  if (provider === "anthropic") {
    return "Anthropic";
  }
  return "auto";
}

function providerDefaultModel(provider = selectedProvider()) {
  if (!state.health) {
    return "";
  }
  const effective = effectiveProvider(state.auth, provider, normalizedModel());
  if (effective === "openai-compatible") {
    return state.health.openai_default_model || "";
  }
  if (effective === "xai") {
    return state.health.xai_default_model || "";
  }
  if (effective === "gemini") {
    return state.health.gemini_default_model || "";
  }
  if (effective === "deepseek") {
    return state.health.deepseek_default_model || "";
  }
  if (effective === "perplexity") {
    return state.health.perplexity_default_model || "";
  }
  return state.health.anthropic_default_model || "";
}

function syncModelInputForProvider(force = false) {
  const current = elements.modelInput.value.trim();
  const previousAuto = elements.modelInput.dataset.autoModel || "";
  const nextDefault = providerDefaultModel(selectedProvider());
  if (!force && current && current !== previousAuto) {
    return;
  }
  elements.modelInput.value = nextDefault;
  elements.modelInput.dataset.autoModel = nextDefault;
}

function syncSystemPromptInput(force = false) {
  const current = elements.systemPromptInput.value.trim();
  const previousAuto = elements.systemPromptInput.dataset.autoPrompt || "";
  const nextDefault = (state.health?.default_system_prompt || "").trim();
  if (!force && current && current !== previousAuto) {
    return;
  }
  elements.systemPromptInput.value = nextDefault;
  elements.systemPromptInput.dataset.autoPrompt = nextDefault;
}

function selectedProviderStatus(auth) {
  const requested = selectedProvider();
  const effective = effectiveProvider(auth, requested, normalizedModel());
  const model = elements.modelInput.value.trim();
  const hasModel = Boolean(model);
  const modelMessage = !hasModel
    ? "Select a provider default model or enter a model id manually."
    : "";
  if (effective === "openai-compatible") {
    return {
      requested,
      effective,
      ready: Boolean(auth?.openai_inference_ready) && hasModel,
      message: hasModel
        ? "Selected provider requires OPENAI_API_KEY. Set OPENAI_BASE_URL if your local server is not on the default /v1 endpoint."
        : modelMessage,
    };
  }
  if (effective === "xai") {
    return {
      requested,
      effective,
      ready: Boolean(auth?.xai_inference_ready) && hasModel,
      message: hasModel ? "Selected provider requires XAI_API_KEY." : modelMessage,
    };
  }
  if (effective === "gemini") {
    return {
      requested,
      effective,
      ready: Boolean(auth?.gemini_inference_ready) && hasModel,
      message: hasModel ? "Selected provider requires GEMINI_API_KEY." : modelMessage,
    };
  }
  if (effective === "deepseek") {
    return {
      requested,
      effective,
      ready: Boolean(auth?.deepseek_inference_ready) && hasModel,
      message: hasModel ? "Selected provider requires DEEPSEEK_API_KEY." : modelMessage,
    };
  }
  if (effective === "perplexity") {
    return {
      requested,
      effective,
      ready: Boolean(auth?.perplexity_inference_ready) && hasModel,
      message: hasModel ? "Selected provider requires PERPLEXITY_API_KEY." : modelMessage,
    };
  }
  return {
    requested,
    effective,
    ready: Boolean(auth?.anthropic_inference_ready) && hasModel,
    message: hasModel
      ? auth?.warning ||
        "Selected provider requires ANTHROPIC_API_KEY, ANTHROPIC_AUTH_TOKEN, or a supported Anthropic auth source."
      : modelMessage,
  };
}

function pillClass(tone) {
  if (tone === "good") return "pill";
  if (tone === "warn") return "pill pill--warn";
  if (tone === "bad") return "pill pill--bad";
  return "pill pill--muted";
}

function updateAuthPanel(auth) {
  state.auth = auth;
  const providerStatus = selectedProviderStatus(auth);

  let tone = "muted";
  let label = "No auth";
  if (providerStatus.ready) {
    tone = "good";
    label = `${providerDisplayName(providerStatus.effective)} ready`;
  } else if (auth.warning && providerStatus.effective === "anthropic") {
    tone = "warn";
    label = auth.active_source === "oauth" ? "OAuth saved" : "Auth warning";
  } else if (
    auth.active_source === "oauth" &&
    auth.saved_oauth_expired &&
    providerStatus.effective === "anthropic"
  ) {
    tone = "warn";
    label = "OAuth expired";
  } else if (auth.authenticated) {
    tone = "warn";
    label = "Auth incomplete";
  }

  elements.authPill.className = pillClass(tone);
  elements.authPill.textContent = label;

  const summaryParts = [];
  if (providerStatus.ready) {
    summaryParts.push(
      `Selected provider ${providerDisplayName(providerStatus.effective)} is ready for inference.`,
    );
  } else if (providerStatus.effective !== "anthropic") {
    summaryParts.push(providerStatus.message);
  } else if (auth.warning && providerStatus.effective === "anthropic") {
    summaryParts.push(auth.warning);
  } else if (auth.active_source === "oauth") {
    summaryParts.push("Requests will use saved Claude OAuth credentials.");
  } else if (auth.active_source === "openai_api_key") {
    summaryParts.push("OPENAI_API_KEY is available for an OpenAI-compatible backend.");
  } else if (auth.active_source === "xai_api_key") {
    summaryParts.push("XAI_API_KEY is available for xAI models.");
  } else if (auth.active_source === "gemini_api_key") {
    summaryParts.push("GEMINI_API_KEY is available for Gemini models.");
  } else if (auth.active_source === "deepseek_api_key") {
    summaryParts.push("DEEPSEEK_API_KEY is available for DeepSeek models.");
  } else if (auth.active_source === "perplexity_api_key") {
    summaryParts.push("PERPLEXITY_API_KEY is available for Perplexity models.");
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
    ["Requested provider", selectedProvider()],
    ["Effective provider", providerStatus.effective],
    ["Provider ready", providerStatus.ready ? "yes" : "no"],
    ["Active source", auth.active_source],
    ["Any inference ready", auth.inference_ready ? "yes" : "no"],
    ["Anthropic ready", auth.anthropic_inference_ready ? "yes" : "no"],
    ["OpenAI-compatible ready", auth.openai_inference_ready ? "yes" : "no"],
    ["xAI ready", auth.xai_inference_ready ? "yes" : "no"],
    ["Gemini ready", auth.gemini_inference_ready ? "yes" : "no"],
    ["DeepSeek ready", auth.deepseek_inference_ready ? "yes" : "no"],
    ["Perplexity ready", auth.perplexity_inference_ready ? "yes" : "no"],
    ["OPENAI_API_KEY", auth.env_openai_api_key ? "yes" : "no"],
    ["XAI_API_KEY", auth.env_xai_api_key ? "yes" : "no"],
    ["GEMINI_API_KEY", auth.env_gemini_api_key ? "yes" : "no"],
    ["DEEPSEEK_API_KEY", auth.env_deepseek_api_key ? "yes" : "no"],
    ["PERPLEXITY_API_KEY", auth.env_perplexity_api_key ? "yes" : "no"],
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
  elements.sendButton.disabled = state.loading || !providerStatus.ready;
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
    ["Anthropic default model", health.anthropic_default_model],
    ["OpenAI-compatible default model", health.openai_default_model || "unset"],
    ["xAI default model", health.xai_default_model],
    ["Gemini default model", health.gemini_default_model],
    ["DeepSeek default model", health.deepseek_default_model],
    ["Perplexity default model", health.perplexity_default_model],
    ["Default system prompt", health.default_system_prompt || "unset"],
  ]
    .map(
      ([term, value]) =>
        `<div class="meta-row"><dt>${escapeHtml(term)}</dt><dd>${escapeHtml(String(value))}</dd></div>`,
    )
    .join("");
  syncModelInputForProvider(true);
  syncSystemPromptInput(true);
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
  renderAssistantPane();
}

function renderAssistantPane() {
  const messages = visibleAssistantMessages();
  if (!messages.length) {
    elements.assistantPane.innerHTML =
      '<article class="message message--empty"><p>No assistant replies yet. Start with a prompt or load an existing session.</p></article>';
    elements.assistantPaneMeta.textContent = "Waiting for a reply.";
    return;
  }

  const latestMessage = messages[messages.length - 1];
  elements.assistantPaneMeta.textContent = latestMessage.usage
    ? `${latestMessage.usage.total_tokens} tokens · ${latestMessage.usage.estimated_cost_usd}`
    : state.liveTurn
      ? "Streaming reply."
      : `${messages.length} assistant ${messages.length === 1 ? "reply" : "replies"}`;

  elements.assistantPane.innerHTML = messages
    .map((message) => {
      const usage = message.usage
        ? `<div class="message__usage">${escapeHtml(
            `${message.usage.total_tokens} tokens · ${message.usage.estimated_cost_usd}`,
          )}</div>`
        : "";
      return `
        <article class="message message--assistant">
          ${usage ? `<div class="message__header">${usage}</div>` : ""}
          <div class="message__blocks">
            ${message.blocks.map(renderBlock).join("")}
          </div>
        </article>
      `;
    })
    .join("");

  elements.assistantPane.scrollTop = elements.assistantPane.scrollHeight;
}

function visibleAssistantMessages() {
  const messages = state.currentSession?.messages
    ? state.currentSession.messages.filter((message) => message.role === "assistant")
    : [];

  if (!state.liveTurn) {
    return messages;
  }

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

function renderInlineMarkdown(text) {
  const placeholders = [];
  let html = escapeHtml(text);

  html = html.replace(/`([^`\n]+)`/g, (_, code) => {
    const placeholder = `%%CODE${placeholders.length}%%`;
    placeholders.push(`<code>${escapeHtml(code)}</code>`);
    return placeholder;
  });

  html = html.replace(/\[([^\]]+)\]\((https?:\/\/[^)\s]+)\)/g, (_, label, href) => {
    return `<a href="${escapeHtml(href)}" target="_blank" rel="noreferrer">${escapeHtml(label)}</a>`;
  });
  html = html.replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>");
  html = html.replace(/__([^_]+)__/g, "<strong>$1</strong>");
  html = html.replace(/(^|[\s(>])\*([^*\n]+)\*(?=[\s).,!?:;]|$)/g, "$1<em>$2</em>");
  html = html.replace(/(^|[\s(>])_([^_\n]+)_(?=[\s).,!?:;]|$)/g, "$1<em>$2</em>");

  for (let index = 0; index < placeholders.length; index += 1) {
    html = html.replace(`%%CODE${index}%%`, placeholders[index]);
  }

  return html;
}

function splitTableRow(line) {
  return line
    .trim()
    .replace(/^\|/, "")
    .replace(/\|$/, "")
    .split("|")
    .map((cell) => cell.trim());
}

function isTableDivider(line) {
  return /^\s*\|?(?:\s*:?-{3,}:?\s*\|)+\s*$/.test(line);
}

function renderMarkdown(text) {
  const normalized = String(text || "").replace(/\r\n/g, "\n").trim();
  if (!normalized) {
    return '<div class="markdown"><p></p></div>';
  }

  const lines = normalized.split("\n");
  const html = [];
  let index = 0;

  const isSpecialBlockStart = (line, nextLine = "") => {
    const trimmed = line.trim();
    return (
      trimmed.startsWith("```") ||
      /^#{1,6}\s+/.test(trimmed) ||
      /^>\s?/.test(trimmed) ||
      /^([-*+]\s+|\d+\.\s+)/.test(trimmed) ||
      /^(-{3,}|\*{3,}|_{3,})$/.test(trimmed) ||
      (trimmed.includes("|") && isTableDivider(nextLine))
    );
  };

  while (index < lines.length) {
    const line = lines[index];
    const trimmed = line.trim();

    if (!trimmed) {
      index += 1;
      continue;
    }

    if (trimmed.startsWith("```")) {
      const language = trimmed.slice(3).trim();
      const codeLines = [];
      index += 1;
      while (index < lines.length && !lines[index].trim().startsWith("```")) {
        codeLines.push(lines[index]);
        index += 1;
      }
      if (index < lines.length) {
        index += 1;
      }
      const languageAttr = language ? ` data-language="${escapeHtml(language)}"` : "";
      html.push(
        `<pre class="markdown__code"><code${languageAttr}>${escapeHtml(codeLines.join("\n"))}</code></pre>`,
      );
      continue;
    }

    const headingMatch = trimmed.match(/^(#{1,6})\s+(.*)$/);
    if (headingMatch) {
      const level = headingMatch[1].length;
      html.push(`<h${level}>${renderInlineMarkdown(headingMatch[2])}</h${level}>`);
      index += 1;
      continue;
    }

    if (trimmed.includes("|") && isTableDivider(lines[index + 1] || "")) {
      const header = splitTableRow(lines[index]);
      index += 2;
      const rows = [];
      while (index < lines.length && lines[index].trim().includes("|")) {
        rows.push(splitTableRow(lines[index]));
        index += 1;
      }
      html.push(`
        <div class="markdown__table-wrap">
          <table>
            <thead>
              <tr>${header.map((cell) => `<th>${renderInlineMarkdown(cell)}</th>`).join("")}</tr>
            </thead>
            <tbody>
              ${rows
                .map(
                  (row) =>
                    `<tr>${row.map((cell) => `<td>${renderInlineMarkdown(cell)}</td>`).join("")}</tr>`,
                )
                .join("")}
            </tbody>
          </table>
        </div>
      `);
      continue;
    }

    if (/^([-*+]\s+|\d+\.\s+)/.test(trimmed)) {
      const ordered = /^\d+\.\s+/.test(trimmed);
      const items = [];
      while (index < lines.length) {
        const itemLine = lines[index].trim();
        const markerPattern = ordered ? /^\d+\.\s+(.*)$/ : /^[-*+]\s+(.*)$/;
        const itemMatch = itemLine.match(markerPattern);
        if (!itemMatch) {
          break;
        }
        items.push(`<li>${renderInlineMarkdown(itemMatch[1])}</li>`);
        index += 1;
      }
      html.push(`<${ordered ? "ol" : "ul"}>${items.join("")}</${ordered ? "ol" : "ul"}>`);
      continue;
    }

    if (/^>\s?/.test(trimmed)) {
      const quoteLines = [];
      while (index < lines.length && /^>\s?/.test(lines[index].trim())) {
        quoteLines.push(lines[index].trim().replace(/^>\s?/, ""));
        index += 1;
      }
      html.push(`<blockquote>${renderInlineMarkdown(quoteLines.join("\n"))}</blockquote>`);
      continue;
    }

    if (/^(-{3,}|\*{3,}|_{3,})$/.test(trimmed)) {
      html.push("<hr />");
      index += 1;
      continue;
    }

    const paragraphLines = [];
    while (index < lines.length) {
      const current = lines[index];
      const next = lines[index + 1] || "";
      if (!current.trim()) {
        break;
      }
      if (paragraphLines.length && isSpecialBlockStart(current, next)) {
        break;
      }
      paragraphLines.push(current);
      index += 1;
    }
    html.push(`<p>${renderInlineMarkdown(paragraphLines.join("\n")).replace(/\n/g, "<br />")}</p>`);
  }

  return `<div class="markdown">${html.join("")}</div>`;
}

function renderBlock(block) {
  if (block.type === "text") {
    return `
      <section class="block">
        ${renderMarkdown(block.text)}
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
  renderAssistantPane();
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
  const providerStatus = selectedProviderStatus(state.auth);
  if (state.auth && !providerStatus.ready) {
    setFlash(
      providerStatus.message,
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
    provider: selectedProvider(),
    system_prompt: elements.systemPromptInput.value.trim() || undefined,
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
  renderAssistantPane();
}

function pushLiveBlock(block) {
  if (!state.liveTurn) {
    return;
  }
  state.liveTurn.blocks.push(block);
  renderAssistantPane();
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
    )}</code> with model <code>${escapeHtml(data.model)}</code> via <code>${escapeHtml(
      data.provider,
    )}</code>.`;
    elements.requestStatus.textContent = `Streaming ${data.provider}`;
    renderSessions();
    return;
  }

  if (name === "assistant_text_delta") {
    if (state.liveTurn) {
      state.liveTurn.assistantText += data.text;
      renderAssistantPane();
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
      renderAssistantPane();
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
    autosizePromptInput();
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
  autosizePromptInput();
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
elements.providerSelect.addEventListener("change", () => {
  syncModelInputForProvider();
  if (state.auth) {
    updateAuthPanel(state.auth);
  }
});
elements.promptInput.addEventListener("keydown", (event) => {
  if (event.isComposing) {
    return;
  }
  if (event.key !== "Enter" || (!event.ctrlKey && !event.metaKey)) {
    return;
  }
  event.preventDefault();
  if (typeof elements.chatForm.requestSubmit === "function") {
    elements.chatForm.requestSubmit();
    return;
  }
  elements.sendButton.click();
});
elements.promptInput.addEventListener("input", () => {
  autosizePromptInput();
});
elements.modelInput.addEventListener("input", () => {
  if (elements.modelInput.value.trim() !== (elements.modelInput.dataset.autoModel || "")) {
    elements.modelInput.dataset.autoModel = "";
  }
  if (state.auth) {
    updateAuthPanel(state.auth);
  }
});
elements.systemPromptInput.addEventListener("input", () => {
  if (elements.systemPromptInput.value.trim() !== (elements.systemPromptInput.dataset.autoPrompt || "")) {
    elements.systemPromptInput.dataset.autoPrompt = "";
  }
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
