/**
 * The security exhibit: a line-art schematic of the local data path —
 * statements hashed and sealed on the Mac, the Keychain holding keys, and
 * consent-gated channels as the only way out. Flat technical drawing in the
 * FlowGraph idiom: hairline strokes, mono captions, animated dash packets,
 * no glow.
 */
export function VaultSchematic() {
  return (
    <>
      <div
        className="vault-exhibit"
        role="img"
        aria-label="Schematic diagram: statement files enter the CanCan process on your Mac, are SHA-256 hashed and verified, then encrypted with Argon2id and SQLCipher into the Vault, which the macOS Keychain unlocks. The only outbound channels are consent-gated: the Gmail API with read-only scope and your own AI provider with your key."
      >
        <svg className="vault-schematic" viewBox="0 0 760 400" aria-hidden="true">
          {/* machine boundary */}
          <rect className="vs-boundary" x="200" y="44" width="536" height="192" />
          <text className="vs-cap" x="212" y="62">CANCAN PROCESS — ON YOUR MAC</text>

          {/* source documents */}
          <text className="vs-cap" x="36" y="60">STATEMENTS</text>
          <path className="vs-doc" d="M36 72h22l12 12v28H36z" />
          <path className="vs-doc" d="M58 72v12h12" />
          <text className="vs-sub" x="53" y="99" textAnchor="middle">PDF</text>
          <path className="vs-doc" d="M36 128h22l12 12v28H36z" />
          <path className="vs-doc" d="M58 128v12h12" />
          <text className="vs-sub" x="53" y="155" textAnchor="middle">CSV</text>
          <path className="vs-doc" d="M36 184h22l12 12v28H36z" />
          <path className="vs-doc" d="M58 184v12h12" />
          <text className="vs-sub" x="53" y="211" textAnchor="middle">IMG</text>

          {/* ingest channels */}
          <path className="vs-edge" d="M70 92C150 92 170 139 246 139" />
          <path className="vs-edge" d="M70 148c70 0 106-9 176-9" />
          <path className="vs-edge" d="M70 204c80 0 96-65 176-65" />
          <path className="vs-packet vpk-a" d="M70 92C150 92 170 139 246 139" />
          <path className="vs-packet vpk-b" d="M70 148c70 0 106-9 176-9" />
          <path className="vs-packet vpk-c" d="M70 204c80 0 96-65 176-65" />

          {/* stage 1 — integrity */}
          <rect className="vs-box" x="246" y="110" width="144" height="58" />
          <text className="vs-label" x="318" y="136" textAnchor="middle">SHA-256</text>
          <text className="vs-sub" x="318" y="153" textAnchor="middle">COPY · VERIFY · DEDUP</text>

          {/* stage 2 — encryption */}
          <path className="vs-edge" d="M390 139h34" />
          <path className="vs-packet vpk-d" d="M390 139h34" />
          <rect className="vs-box" x="424" y="110" width="144" height="58" />
          <text className="vs-label" x="496" y="136" textAnchor="middle">ENCRYPT</text>
          <text className="vs-sub" x="496" y="153" textAnchor="middle">ARGON2ID · SQLCIPHER</text>

          {/* vault */}
          <path className="vs-edge" d="M568 139h18" />
          <path className="vs-packet vpk-e" d="M568 139h18" />
          <rect className="vs-vault" x="586" y="98" width="140" height="82" />
          <rect className="vs-vault-inner" x="592" y="104" width="128" height="70" />
          <text className="vs-title" x="656" y="134" textAnchor="middle">VAULT</text>
          <text className="vs-accent" x="656" y="150" textAnchor="middle">ENCRYPTED AT REST</text>
          <text className="vs-sub" x="656" y="163" textAnchor="middle">DB + DOCUMENTS</text>

          {/* renderer boundary note */}
          <text className="vs-cap" x="466" y="224" textAnchor="middle">
            UI SEES READ MODELS ONLY — NO RAW BYTES · NO PATHS · NO HANDLES
          </text>

          {/* keychain */}
          <path className="vs-edge" d="M656 286V180" />
          <path className="vs-packet vpk-f" d="M656 286V180" />
          <text className="vs-sub" x="646" y="240" textAnchor="end">UNLOCKS</text>
          <rect className="vs-box" x="586" y="286" width="140" height="52" />
          <text className="vs-label" x="656" y="309" textAnchor="middle">KEYCHAIN</text>
          <text className="vs-sub" x="656" y="324" textAnchor="middle">KEYS · TOKENS</text>

          {/* consent-gated outbound channels */}
          <path className="vs-net" d="M312 236c0 56-62 80-116 94" />
          <rect className="vs-gate" x="264" y="282" width="12" height="12" />
          <text className="vs-accent" x="256" y="291" textAnchor="end">CONSENT</text>
          <rect className="vs-box" x="76" y="318" width="120" height="44" />
          <text className="vs-label" x="136" y="338" textAnchor="middle">GMAIL API</text>
          <text className="vs-sub" x="136" y="353" textAnchor="middle">READ-ONLY SCOPE</text>

          <path className="vs-net" d="M490 236c0 56-52 80-146 94" />
          <rect className="vs-gate" x="456" y="282" width="12" height="12" />
          <text className="vs-accent" x="448" y="291" textAnchor="end">CONSENT</text>
          <rect className="vs-box" x="276" y="318" width="136" height="44" />
          <text className="vs-label" x="344" y="338" textAnchor="middle">AI PROVIDER</text>
          <text className="vs-sub" x="344" y="353" textAnchor="middle">YOUR KEY · YOUR CALL</text>
        </svg>
      </div>
      <p className="pair-caption mono-tag">
        [ No listener, no telemetry — updates verify signed metadata before anything runs ]
      </p>
    </>
  );
}
