# rex-client

Next.js client for the `rex` PDF search & navigator server.

## Stack

- Next.js 15 (App Router)
- React 19
- Tailwind CSS 4
- TypeScript
- pnpm

## Setup

```bash
pnpm install
cp .env.example .env.local   # point REX_API_BASE at your rex server
pnpm dev
```

The dev server proxies `/v1/*` to `REX_API_BASE` (default `http://localhost:8080`) via Next.js rewrites — see `next.config.ts`.

## Scripts

- `pnpm dev` — local dev server
- `pnpm build` — production build
- `pnpm start` — run production build
- `pnpm lint` — eslint
- `pnpm typecheck` — `tsc --noEmit`

## Structure

```
src/
├── app/                  App Router pages
├── components/           UI components
└── lib/
    ├── rex.ts            API client
    └── types.ts          Shared types mirroring rex-domain
```

The API client targets the endpoints described in `docs/superpowers/specs/2026-05-15-rex-pdf-search-server-design.md` §8.
