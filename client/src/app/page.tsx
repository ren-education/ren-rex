import { SearchPanel } from "@/components/search-panel";
import { listSubjects } from "@/lib/rex";

export const dynamic = "force-dynamic";

export default async function Home() {
  const subjects = await listSubjects().catch(() => []);

  return (
    <main className="mx-auto flex min-h-screen max-w-5xl flex-col gap-8 px-6 py-12">
      <header className="flex flex-col gap-2">
        <h1 className="text-3xl font-semibold tracking-tight">rex</h1>
        <p className="text-sm text-neutral-500 dark:text-neutral-400">
          Search questions, notes, and PDF pages across subjects.
        </p>
      </header>

      <SearchPanel subjects={subjects} />
    </main>
  );
}
