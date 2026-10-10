export interface Asset { id: number; title: string; kind: 'caption' | 'resource'; lessonId: number | null }
export interface Lesson { id: number; title: string; section: string; path: string; position: number; duration: number; completed: boolean; missing: boolean; assets: Asset[] }
export interface Course { id: number; title: string; path: string; lessons: Lesson[]; resources: Asset[]; lastLessonId: number | null; updatedAt: number }
export interface Note { id: number; lessonId: number; position: number; text: string; bookmark: boolean }
export interface Job { id: number; lessonId: number; title: string; status: string; stage: string; progress: number; error: string | null }
export interface Settings { libraryRoot: string; modelPath: string; speed: number; autoplay: boolean; theme: 'system' | 'light' | 'dark'; language: import('./i18n').Language }
export interface PreviewAsset { path: string; title: string; kind: string }
export interface Preview { path: string; title: string; warnings: string[]; resources: PreviewAsset[]; sections: { path: string; title: string; lessons: { path: string; title: string; captions: PreviewAsset[]; resources: PreviewAsset[] }[] }[] }

export interface Progress { id: number; position: number; duration: number; completed: boolean | null }
export interface SearchResult { kind: 'course' | 'lesson' | 'note' | 'caption'; id: number; courseId: number; courseTitle: string; lessonId: number | null; lessonTitle: string | null; position: number | null; snippet: string; missing: boolean }
export interface LessonTarget { lessonId: number | null; position: number | null }
export interface ResourceInfo { preview: 'pdf' | 'text' | 'image' | 'external'; size: number }
export interface CaptionCue { position: number; end: number; text: string }
