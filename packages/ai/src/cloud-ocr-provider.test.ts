import { describe, expect, it } from "vitest";

import {
  buildCloudOcrRequest,
  CLOUD_OCR_PROMPT_VERSION,
  CLOUD_OCR_TRANSCRIPTION_PROMPT,
  CloudOcrContractError,
  parseCloudOcrResponse,
  selectCloudOcrConfig,
} from "./cloud-ocr-provider";

const analyzerConfig = {
  endpointUrl: "https://analyse.example.test/v1/chat/completions",
  model: "analysis-model",
  apiKey: "analyser-secret",
};

const dedicatedOcrConfig = {
  endpointUrl: "https://ocr.example.test/v1/chat/completions",
  model: "ocr-model",
  apiKey: "ocr-secret",
};

const pngDataUrl = "data:image/png;base64,c3ludGhldGljLXBuZw==";
const expectedPrompt =
  "Transcribe every visible text element in natural reading order; preserve language, numbers, punctuation, case, and line breaks; use ? only for unreadable single characters; no summary, translation, correction, explanation, Markdown, JSON, coordinates, boxes, or labels.";

describe("cloud OCR provider contract", () => {
  it("uses the complete dedicated OCR configuration when present", () => {
    expect(
      selectCloudOcrConfig({ analyser: analyzerConfig, dedicatedOcr: dedicatedOcrConfig }),
    ).toEqual(dedicatedOcrConfig);
  });

  it("falls back to the complete analyser configuration without field mixing", () => {
    expect(selectCloudOcrConfig({ analyser: analyzerConfig })).toEqual(analyzerConfig);
  });

  it("rejects an incomplete dedicated configuration instead of mixing it with analyser fields", () => {
    expect(() =>
      selectCloudOcrConfig({
        analyser: analyzerConfig,
        dedicatedOcr: { ...dedicatedOcrConfig, apiKey: "" },
      }),
    ).toThrow(CloudOcrContractError);
  });

  it("builds the exact OpenAI-compatible Chat Completions request for one PNG", () => {
    const request = buildCloudOcrRequest(
      { analyser: analyzerConfig, dedicatedOcr: dedicatedOcrConfig },
      pngDataUrl,
    );

    expect(CLOUD_OCR_PROMPT_VERSION).toBe("cloud-ocr-transcription-v1");
    expect(CLOUD_OCR_TRANSCRIPTION_PROMPT).toBe(expectedPrompt);
    expect(request.url).toBe(dedicatedOcrConfig.endpointUrl);
    expect(request.init).toEqual({
      method: "POST",
      headers: {
        Authorization: `Bearer ${dedicatedOcrConfig.apiKey}`,
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        model: dedicatedOcrConfig.model,
        messages: [
          {
            role: "user",
            content: [
              { type: "text", text: expectedPrompt },
              { type: "image_url", image_url: { url: pngDataUrl } },
            ],
          },
        ],
      }),
    });
  });

  it("keeps the API key out of the body and out of contract errors", () => {
    const request = buildCloudOcrRequest({ analyser: analyzerConfig }, pngDataUrl);
    const body = request.init.body;

    expect(request.init.headers).toEqual({
      Authorization: `Bearer ${analyzerConfig.apiKey}`,
      "Content-Type": "application/json",
    });
    expect(body).not.toContain(analyzerConfig.apiKey);

    try {
      buildCloudOcrRequest({ analyser: analyzerConfig }, "data:image/jpeg;base64,not-a-png");
      throw new Error("Expected the invalid data URL to fail.");
    } catch (error) {
      expect(error).toBeInstanceOf(CloudOcrContractError);
      expect((error as Error).message).not.toContain(analyzerConfig.apiKey);
      expect((error as Error).message).not.toContain(body);
    }
  });

  it("returns plain text from choices[0].message.content without changing line breaks", () => {
    expect(
      parseCloudOcrResponse({
        choices: [{ message: { content: "第一行\nAmount: SGD 10.00" } }],
      }),
    ).toBe("第一行\nAmount: SGD 10.00");
  });

  it("rejects malformed, empty, and non-text Chat Completions responses", () => {
    expect(() => parseCloudOcrResponse({ choices: [] })).toThrow(CloudOcrContractError);
    expect(() => parseCloudOcrResponse({ choices: [{ message: { content: "" } }] })).toThrow(
      CloudOcrContractError,
    );
    expect(() => parseCloudOcrResponse({ choices: [{ message: { content: ["text"] } }] })).toThrow(
      CloudOcrContractError,
    );
  });
});
