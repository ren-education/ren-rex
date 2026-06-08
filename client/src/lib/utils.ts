import { clsx, type ClassValue } from "clsx"
import { twMerge } from "tailwind-merge"

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}

/**
 * Encode a root-relative file path for the `/v1/files/*path` endpoint:
 * percent-encode each segment (spaces, parens, etc.) but keep the slashes
 * so the axum `*path` wildcard captures the full path.
 */
export function encodeFilePath(path: string): string {
  return path.split("/").map(encodeURIComponent).join("/")
}
