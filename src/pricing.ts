/** Prices for models that appear in Grok Build.
 *  Grok: official xAI API list prices
 *  Composer: Cursor list prices
 */

export type ModelPrice = {
  id: string
  label: string
  inputPerM: number
  outputPerM: number
  /** Informational only; logs do not expose cache hits reliably */
  cachedInputPerM?: number
  context?: string
  note?: string
}

/** Models that appear in Grok Build (region-dependent). */
export const OFFICIAL_PRICES: ModelPrice[] = [
  {
    id: "grok-4.5",
    label: "Grok 4.5",
    inputPerM: 2.0,
    outputPerM: 6.0,
    cachedInputPerM: 0.5,
    context: "500k",
    note: "xAI API · default in USA (elsewhere often via VPN)",
  },
  {
    id: "grok-build",
    label: "grok-build",
    inputPerM: 1.0,
    outputPerM: 2.0,
    context: "256k",
    note: "xAI Code API · extra option in some regions (e.g. EU)",
  },
  {
    id: "grok-build-0.1",
    label: "grok-build-0.1",
    inputPerM: 1.0,
    outputPerM: 2.0,
    context: "256k",
    note: "Official Code API id (same rates as grok-build)",
  },
  {
    id: "grok-composer-2.5-fast",
    label: "Composer 2.5 Fast",
    inputPerM: 3.0,
    outputPerM: 15.0,
    context: "200k",
    note: "Cursor list price · default outside USA",
  },
  {
    id: "grok-composer-2.5",
    label: "Composer 2.5 Standard",
    inputPerM: 0.5,
    outputPerM: 2.5,
    context: "200k",
    note: "Cursor list price · Standard tier",
  },
]

const byId = new Map(OFFICIAL_PRICES.map((p) => [p.id.toLowerCase(), p]))

const UNPRICED: ModelPrice = {
  id: "unknown",
  label: "Unknown",
  inputPerM: 0,
  outputPerM: 0,
  note: "Not a known Grok Build model / no public list price",
}

export function resolvePrice(modelId: string | null | undefined): ModelPrice {
  if (!modelId) return { ...UNPRICED }

  const key = modelId.toLowerCase().trim()
  const exact = byId.get(key)
  if (exact) return exact

  if (key.startsWith("grok-build")) return byId.get("grok-build")!
  if (key.startsWith("grok-4.5")) return byId.get("grok-4.5")!

  if (key.includes("composer")) {
    if (key.includes("standard") && !key.includes("fast")) {
      return byId.get("grok-composer-2.5")!
    }
    return byId.get("grok-composer-2.5-fast")!
  }

  return { ...UNPRICED, id: modelId, label: modelId }
}

export type CostBreakdown = {
  inputCost: number
  outputCost: number
  totalCost: number
  priced: boolean
  price: ModelPrice
}

export function calcCost(
  modelId: string | null | undefined,
  inputTokens: number,
  outputTokens: number,
): CostBreakdown {
  const price = resolvePrice(modelId)
  const priced = price.inputPerM > 0 || price.outputPerM > 0
  const inputCost = (inputTokens / 1_000_000) * price.inputPerM
  const outputCost = (outputTokens / 1_000_000) * price.outputPerM
  return {
    inputCost,
    outputCost,
    totalCost: inputCost + outputCost,
    priced,
    price,
  }
}

export const PRICING_SOURCE = "https://docs.x.ai/developers/pricing"
export const PRICING_COMPOSER = "https://cursor.com/docs"
export const PRICING_UPDATED = "2026-07"
