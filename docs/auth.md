---
title: "Authentication Guide"
description: "Authentication methods, credential storage, and error handling for developers and AI agents using Storage CLI."
author: "SpaceCorps"
date: "2026-09-24"
---

# Authentication Guide for Storage CLI

This document outlines authentication methods, credential storage, and error handling for developers and AI agents using the Storage CLI.

## Overview

Storage CLI supports two flexible authentication mechanisms for uploading to Azure Blob Storage:
1. **Azure CLI Session (`az login`)**: If no account key is stored, the CLI automatically invokes Azure CLI (`az storage container generate-sas`) to produce time-limited upload and read SAS tokens.
2. **Native OS Keystore**: You can store Azure Storage account keys or SAS tokens directly in the host OS vault (macOS Keychain, Windows DPAPI, Linux Secret Service).

## Authentication Flows

### Profile Configuration with Azure CLI
When you are logged into Azure CLI via `az login`, you only need to register the profile name and storage account name:

```bash
storage accounts add ivy-tendril --account-name stivytelemetry --container ivy-tendril
```

### Profile Configuration with Account Key or SAS Token
To store credentials securely in the OS keystore:

```bash
# Prompted securely without echo:
storage accounts add ivy-tendril --account-name stivytelemetry --container ivy-tendril

# Non-interactive stdin pipe (e.g. in CI or agent runners):
echo "$AZURE_STORAGE_KEY" | storage accounts add ivy-tendril --account-name stivytelemetry --container ivy-tendril --key-stdin
```

### Environment Variable Bypass
For ephemeral containers or mock testing:
- `STORAGE_SAS_TOKEN`: Provide a pre-generated SAS token string directly.
- `STORAGE_ENDPOINT`: Override the target Azure Blob endpoint.
