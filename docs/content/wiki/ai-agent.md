---
title: AI Agent
category: AI Agent
tags: [agent, providers, configuration]
---

Granit includes an integrated AI agent that runs alongside your notes. It connects to a language-model provider of your choice, streams its responses into the chat panel, and can either answer questions about your cave or act on it directly. This page covers choosing a provider, selecting a model, and the difference between the two operating modes. For the tool catalog and retrieval settings, see [[agent-tools-and-rag]].

# Choosing a provider

The agent supports four kinds of providers. You configure one or more of them and pick which is active; each provider entry can be given an optional name to tell similar entries apart.

- **Ollama** — a local model server running on your own machine. Free, private, and offline. Defaults to `http://localhost:11434`; set a custom base URL if Ollama runs elsewhere. No API key required.
- **Anthropic** — Claude models via the Anthropic API. Requires an API key.
- **Mistral** — Mistral models via the Mistral API. Requires an API key.
- **OpenAI / ChatGPT-compatible** — any endpoint that speaks the OpenAI API. Set a custom base URL plus an API key. This covers OpenAI itself as well as the many self-hosted and third-party servers that expose an OpenAI-compatible interface.

> [!TIP]
> Ollama is the zero-cost option. It runs entirely on your hardware, needs no API key, and keeps your notes on your machine. If you want to try the agent without signing up for a paid service, install Ollama, pull a model, and point Granit at it.

> [!NOTE]
> Provider settings, including API keys, are stored per cave in that cave's `.granit/config.yml`. Keys are not shared between caves and are never written into your note files. See [[configuration]] for the full settings layout.

# Selecting a model

Once a provider is active, Granit queries it for the list of models it offers and presents them for selection. The choices are therefore dynamic: with Ollama you see the models you have pulled locally, and with a hosted provider you see the models that account can access. If no model is selected, the agent falls back to a provider-specific default.

# Ask mode and Agent mode

The agent operates in one of two modes, switchable from the chat panel.

- **Ask mode** is for questions about your existing notes. Relevant notes are retrieved from your cave and injected into the conversation as context, so the model can answer from what you have written. Ask mode is read-only — it does not modify your cave.
- **Agent mode** gives the model the full toolset, so it can create, edit, move, and delete notes and folders, manage daily notes and todos, search, and fetch from the web. No retrieval context is injected in Agent mode; the model gathers what it needs through tools instead.

Retrieval context (RAG) is injected only in Ask mode. The retrieval mechanism and the available tools are described in [[agent-tools-and-rag]].

# Streaming responses

The agent streams its output. Text appears incrementally as the model generates it, and in Agent mode each tool call and its result are shown in the chat as they happen, so you can follow what the agent is doing in real time.
