/**
 * Gallery tile shape used by the home page when real photos exist.
 * Target may be empty on a real row; the page widens `target` locally.
 */
export interface Photo {
  slug: string;
  target: string;
  ratio: number; // width / height
  integration: string;
  photographer: string;
  photographerSlug: string;
  camera: string;
  /** Optional real thumbnail URL; when present, the Photo component renders the image */
  thumbSrc?: string;
}
