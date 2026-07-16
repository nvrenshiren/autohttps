import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

/** shadcn 约定:合并 Tailwind 类(去重冲突)。 */
export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}
