// Optional cloud image OCR adapter and transport executor (spec 0004). The
// adapter plus injected-fetch executor are approved; credential storage,
// app/Tauri/sidecar/UI wiring, request logging, live provider calls, and
// provider-specific parsing are explicitly out of scope.
export const CLOUD_OCR_PROMPT_VERSION = "cloud-ocr-transcription-v1";

export const CLOUD_OCR_TRANSCRIPTION_PROMPT =
  "Transcribe every visible text element in natural reading order; preserve language, numbers, punctuation, case, and line breaks; use ? only for unreadable single characters; no summary, translation, correction, explanation, Markdown, JSON, coordinates, boxes, or labels.";

export interface OpenAiCompatibleChatCompletionsConfig {
  endpointUrl: string;
  model: string;
  apiKey: string;
}

export interface CloudOcrProviderConfiguration {
  analyser: OpenAiCompatibleChatCompletionsConfig;
  dedicatedOcr?: OpenAiCompatibleChatCompletionsConfig;
}

export interface CloudOcrRequest {
  url: string;
  init: {
    method: "POST";
    headers: {
      Authorization: string;
      "Content-Type": "application/json";
    };
    body: string;
  };
}

export class CloudOcrContractError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "CloudOcrContractError";
  }
}

export function selectCloudOcrConfig(
  configuration: CloudOcrProviderConfiguration,
): OpenAiCompatibleChatCompletionsConfig {
  if (!isRecord(configuration)) {
    throw new CloudOcrContractError("Cloud OCR configuration must be a complete endpoint, model, and API key.");
  }

  return requireCompleteConfig(
    configuration.dedicatedOcr === undefined
      ? configuration.analyser
      : configuration.dedicatedOcr,
  );
}

export function buildCloudOcrRequest(
  configuration: CloudOcrProviderConfiguration,
  boundedPngDataUrl: string,
): CloudOcrRequest {
  const config = selectCloudOcrConfig(configuration);
  requireBoundedPngDataUrl(boundedPngDataUrl);

  return {
    url: config.endpointUrl,
    init: {
      method: "POST",
      headers: {
        Authorization: `Bearer ${config.apiKey}`,
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        model: config.model,
        messages: [
          {
            role: "user",
            content: [
              { type: "text", text: CLOUD_OCR_TRANSCRIPTION_PROMPT },
              { type: "image_url", image_url: { url: boundedPngDataUrl } },
            ],
          },
        ],
      }),
    },
  };
}

export function parseCloudOcrResponse(response: unknown): string {
  if (!isRecord(response) || !Array.isArray(response.choices)) {
    throw new CloudOcrContractError("Cloud OCR response did not contain transcription text.");
  }

  const firstChoice = response.choices[0];
  if (!isRecord(firstChoice) || !isRecord(firstChoice.message)) {
    throw new CloudOcrContractError("Cloud OCR response did not contain transcription text.");
  }

  const content = firstChoice.message.content;
  if (typeof content !== "string" || content.trim().length === 0) {
    throw new CloudOcrContractError("Cloud OCR response did not contain transcription text.");
  }

  return content;
}

export async function executeCloudOcr(
  configuration: CloudOcrProviderConfiguration,
  boundedPngDataUrl: string,
  fetchImpl: typeof fetch = globalThis.fetch,
): Promise<string> {
  const request = buildCloudOcrRequest(configuration, boundedPngDataUrl);

  let response: Response;
  try {
    response = await fetchImpl(request.url, request.init);
  } catch {
    throw new CloudOcrContractError("Cloud OCR request failed.");
  }

  if (!response.ok) {
    throw new CloudOcrContractError("Cloud OCR provider rejected the request.");
  }

  let body: unknown;
  try {
    body = await response.json();
  } catch {
    throw new CloudOcrContractError("Cloud OCR response was not valid JSON.");
  }

  return parseCloudOcrResponse(body);
}

function requireCompleteConfig(value: unknown): OpenAiCompatibleChatCompletionsConfig {
  if (
    !isRecord(value) ||
    !isNonEmptyString(value.endpointUrl) ||
    !isNonEmptyString(value.model) ||
    !isNonEmptyString(value.apiKey)
  ) {
    throw new CloudOcrContractError("Cloud OCR configuration must be a complete endpoint, model, and API key.");
  }

  return {
    endpointUrl: value.endpointUrl,
    model: value.model,
    apiKey: value.apiKey,
  };
}

function requireBoundedPngDataUrl(value: string): void {
  const prefix = "data:image/png;base64,";
  if (!value.startsWith(prefix) || value.length === prefix.length) {
    throw new CloudOcrContractError("Cloud OCR requires one bounded PNG data URL.");
  }
}

function isNonEmptyString(value: unknown): value is string {
  return typeof value === "string" && value.trim().length > 0;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && !Array.isArray(value) && typeof value === "object";
}
