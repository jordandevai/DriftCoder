const extensionMap: Record<string, string> = {
	// JavaScript/TypeScript
	js: 'javascript',
	mjs: 'javascript',
	cjs: 'javascript',
	jsx: 'javascript',
	ts: 'typescript',
	tsx: 'typescript',
	mts: 'typescript',
	cts: 'typescript',

	// Web
	html: 'html',
	htm: 'html',
	css: 'css',
	scss: 'css',
	sass: 'css',
	less: 'css',

	// Data formats
	json: 'json',
	jsonc: 'json',
	yaml: 'yaml',
	yml: 'yaml',
	toml: 'toml',
	xml: 'xml',
	svg: 'xml',

	// Python
	py: 'python',
	pyw: 'python',
	pyi: 'python',

	// Rust
	rs: 'rust',

	// Go
	go: 'go',

	// Java/Kotlin
	java: 'java',
	kt: 'kotlin',
	kts: 'kotlin',

	// C/C++
	c: 'c',
	h: 'c',
	cpp: 'cpp',
	cxx: 'cpp',
	cc: 'cpp',
	hpp: 'cpp',
	hxx: 'cpp',

	// Shell
	sh: 'shell',
	bash: 'shell',
	zsh: 'shell',
	fish: 'shell',

	// SQL
	sql: 'sql',

	// Markdown
	md: 'markdown',
	mdx: 'markdown',

	// Config files
	conf: 'shell',
	cfg: 'ini',
	ini: 'ini',
	env: 'shell',

	// Docker
	dockerfile: 'dockerfile',

	// Misc
	txt: 'text',
	log: 'text',
	csv: 'text',
	gitignore: 'shell',
	dockerignore: 'shell'
};

const filenameMap: Record<string, string> = {
	Dockerfile: 'dockerfile',
	Makefile: 'makefile',
	Cargo: 'toml',
	'package.json': 'json',
	'tsconfig.json': 'json',
	'.gitignore': 'shell',
	'.dockerignore': 'shell',
	'.env': 'shell',
	'.env.local': 'shell',
	'.env.development': 'shell',
	'.env.production': 'shell'
};

/**
 * Detect the language/mode for a file based on its name
 */
export function detectLanguage(filename: string): string {
	// Check exact filename matches first
	if (filename in filenameMap) {
		return filenameMap[filename];
	}

	// Check by extension
	const ext = filename.split('.').pop()?.toLowerCase();
	if (ext && ext in extensionMap) {
		return extensionMap[ext];
	}

	// Default to plain text
	return 'text';
}

/**
 * Get a human-readable language name
 */
export function getLanguageLabel(language: string): string {
	const labels: Record<string, string> = {
		javascript: 'JavaScript',
		typescript: 'TypeScript',
		html: 'HTML',
		css: 'CSS',
		json: 'JSON',
		yaml: 'YAML',
		xml: 'XML',
		python: 'Python',
		rust: 'Rust',
		go: 'Go',
		java: 'Java',
		kotlin: 'Kotlin',
		c: 'C',
		cpp: 'C++',
		shell: 'Shell',
		sql: 'SQL',
		markdown: 'Markdown',
		dockerfile: 'Dockerfile',
		makefile: 'Makefile',
		toml: 'TOML',
		ini: 'INI',
		text: 'Plain Text'
	};

	return labels[language] || language.charAt(0).toUpperCase() + language.slice(1);
}
