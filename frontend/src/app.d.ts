declare global {
  namespace App {
    interface Locals {
      user: { id: string; displayName: string } | null;
    }
    interface PageData {
      user: { id: string; displayName: string } | null;
    }
  }
}

export {};
