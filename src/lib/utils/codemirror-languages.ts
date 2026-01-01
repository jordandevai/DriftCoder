import type { Extension } from '@codemirror/state';

type LanguageLoader = () => Promise<Extension>;

const languageLoaders: Record<string, LanguageLoader> = {
	javascript: async () => (await import('@codemirror/lang-javascript')).javascript(),
	typescript: async () => (await import('@codemirror/lang-javascript')).javascript({ typescript: true }),
	jsx: async () => (await import('@codemirror/lang-javascript')).javascript({ jsx: true }),
	tsx: async () =>
		(await import('@codemirror/lang-javascript')).javascript({ jsx: true, typescript: true }),
	python: async () => (await import('@codemirror/lang-python')).python(),
	rust: async () => (await import('@codemirror/lang-rust')).rust(),
	html: async () => (await import('@codemirror/lang-html')).html(),
	css: async () => (await import('@codemirror/lang-css')).css(),
	json: async () => (await import('@codemirror/lang-json')).json(),
	markdown: async () => (await import('@codemirror/lang-markdown')).markdown(),
	yaml: async () => (await import('@codemirror/lang-yaml')).yaml(),
	xml: async () => (await import('@codemirror/lang-xml')).xml(),
	sql: async () => (await import('@codemirror/lang-sql')).sql()
};

export async function loadLanguageExtension(language: string): Promise<Extension> {
	const loader = languageLoaders[language];
	if (!loader) return [];
	try {
		return await loader();
	} catch (error) {
		console.error(`Failed to load language extension: ${language}`, error);
		return [];
	}
}

