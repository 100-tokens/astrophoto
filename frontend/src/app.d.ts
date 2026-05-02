declare global {
  namespace App {
    interface Locals {
      user: {
        id: string;
        displayName: string;
        following_ids: string[];
      } | null;
    }
    interface PageData {
      user: { id: string; displayName: string; following_ids: string[] } | null;
    }
  }
}

export {};
