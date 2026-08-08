/**
 * Dev-only gallery for the slice-1 foundation: every primitive and pattern on
 * one Ledger surface, plus a Vault-dark strip for the accent usage rule.
 * Rendered by preview.tsx (`?state=primitives`); never shipped.
 */
import {
  ActionBar,
  Badge,
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogDescription,
  DialogTitle,
  DialogTrigger,
  EmptyState,
  Icon,
  Input,
  MetricRow,
  MonogramTile,
  Panel,
  SectionHeader,
  Select,
  Skeleton,
  StatusPoint,
  Tooltip,
} from "@cancan/ui";

function Row({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="border-b border-ledger-rule py-4 last:border-b-0">
      <p className="font-mono text-xs uppercase tracking-mono-label text-ledger-text-muted">
        {label}
      </p>
      <div className="mt-3 flex flex-wrap items-center gap-2">{children}</div>
    </div>
  );
}

export function PrimitivesGallery({ dialogOpen = false }: { dialogOpen?: boolean }) {
  return (
    <div className="min-h-full bg-ledger-mineral px-10 py-8 font-sans text-ledger-ink">
      <p className="font-mono text-xs uppercase tracking-mono-label text-ledger-text-muted">
        Slice 1 foundation
      </p>
      <h1 className="mt-2 font-serif text-2xl font-display text-ledger-ink">
        Your money, organized
      </h1>
      <p className="mt-1 text-sm text-ledger-text-muted">
        Fraunces display moment (≤1.5rem, 560) over a Geologica body — tokens, primitives, patterns.
      </p>

      <div className="mt-8 grid max-w-4xl grid-cols-1 gap-6">
        <Panel className="px-5 py-2">
          <Row label="Buttons">
            <Button>Primary</Button>
            <Button variant="strong">Strong</Button>
            <Button variant="quiet">Quiet</Button>
            <Button variant="text">Text</Button>
            <Button variant="danger">Danger</Button>
            <Button disabled size="sm">
              Busy sm
            </Button>
            <Button icon={<Icon name="check" size={16} />} size="sm" variant="quiet">
              With icon
            </Button>
          </Row>
          <Row label="Inputs + select">
            <div className="w-56">
              <Input placeholder="Plain input" />
            </div>
            <div className="w-40">
              <Input numeric placeholder="12,456.78" />
            </div>
            <div className="w-48">
              <Select
                ariaLabel="Account decision"
                options={[
                  { value: "accept", label: "Accept" },
                  { value: "dismiss", label: "Dismiss" },
                ]}
                placeholder="Choose"
              />
            </div>
            <Tooltip content="Evidence stays encrypted on this Mac">
              <Button variant="text">Hover for tooltip</Button>
            </Tooltip>
          </Row>
          <Row label="Badges, status points, monograms">
            <Badge tone="attention">3</Badge>
            <Badge tone="healthy">12</Badge>
            <Badge>SGD</Badge>
            <StatusPoint tone="healthy" />
            <StatusPoint tone="attention" />
            <StatusPoint tone="risk" />
            <StatusPoint tone="idle" />
            <MonogramTile name="DBS Multiplier" />
            <MonogramTile name="Wise USD Balance" size={32} />
          </Row>
        </Panel>

        <Panel className="px-5 py-4">
          <SectionHeader
            action={
              <Button size="sm" variant="text">
                View all
              </Button>
            }
            count={5}
            title="Tasks"
            tone="attention"
          />
          <div className="mt-2">
            <MetricRow label="DBS Multiplier Account" meta="As of 19 Jul 2026" value="SGD 12,456.78" />
            <MetricRow label="Wise USD Balance" meta="As of 18 Jul 2026" ruled={false} value="USD 980.50" />
          </div>
          <ActionBar align="end" className="mt-4 border-t border-ledger-rule pt-4">
            <Button variant="text">Keep unassigned</Button>
            <Button variant="quiet">Choose an existing source</Button>
            <Button>Create source and continue</Button>
          </ActionBar>
        </Panel>

        <Panel>
          <EmptyState
            action={<Button variant="quiet">Add evidence</Button>}
            body="Save a statement to your CanCan Inbox and it appears here, encrypted on this Mac."
            title="No evidence yet"
          />
        </Panel>

        <Panel className="px-5 py-4">
          <Row label="Loading">
            <Skeleton className="h-3 w-40" />
            <Skeleton className="h-3 w-24" />
            <Skeleton className="size-7" />
          </Row>
          <div className="py-4">
            <Dialog defaultOpen={dialogOpen}>
              <DialogTrigger asChild>
                <Button variant="quiet">Open dialog</Button>
              </DialogTrigger>
              <DialogContent>
                <DialogTitle>Statement locked</DialogTitle>
                <DialogDescription>
                  This PDF needs its statement password once. CanCan keeps it in this Mac's
                  Keychain, scoped to the source.
                </DialogDescription>
                <Input aria-label="Statement password" className="mt-4" placeholder="Password" type="password" />
                <DialogActions>
                  <Button variant="text">Cancel</Button>
                  <Button>Unlock</Button>
                </DialogActions>
              </DialogContent>
            </Dialog>
          </div>
        </Panel>

        <div className="rounded-md bg-vault-obsidian px-5 py-4 text-vault-text">
          <p className="font-mono text-xs uppercase tracking-mono-label text-vault-text-muted">
            Vault-dark strip — accent rule
          </p>
          <div className="mt-3 flex flex-wrap items-center gap-3">
            <span className="text-sm text-accent-go-bright">go-bright on obsidian (10.17:1)</span>
            <span className="text-sm text-accent-go">go on obsidian (7.19:1)</span>
            <span className="rounded-sm bg-accent-go-bright px-2 py-1 text-xs font-medium text-accent-go-ink">
              go-ink on go-bright (9.10:1)
            </span>
            <StatusPoint tone="healthy" />
            <Icon className="text-accent-go-bright" name="lock" size={18} />
          </div>
        </div>
      </div>
    </div>
  );
}
