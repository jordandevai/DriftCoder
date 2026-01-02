export type HostKeyContext =
	| {
			host: string;
			port: number;
			keyType: string;
			fingerprintSha256: string;
			publicKeyOpenssh: string;
	  }
	| {
			host: string;
			port: number;
			keyType: string;
			expectedFingerprintSha256: string;
			actualFingerprintSha256: string;
			expectedPublicKeyOpenssh: string;
			actualPublicKeyOpenssh: string;
	  };

function isRecord(value: unknown): value is Record<string, unknown> {
	return !!value && typeof value === 'object';
}

export function parseHostKeyContext(context: unknown): HostKeyContext | null {
	if (!isRecord(context)) return null;
	if (typeof context.host !== 'string') return null;
	if (typeof context.port !== 'number') return null;
	if (typeof context.keyType !== 'string') return null;

	// untrusted
	if (typeof context.fingerprintSha256 === 'string' && typeof context.publicKeyOpenssh === 'string') {
		return {
			host: context.host,
			port: context.port,
			keyType: context.keyType,
			fingerprintSha256: context.fingerprintSha256,
			publicKeyOpenssh: context.publicKeyOpenssh
		};
	}

	// mismatch
	if (
		typeof context.expectedFingerprintSha256 === 'string' &&
		typeof context.actualFingerprintSha256 === 'string' &&
		typeof context.expectedPublicKeyOpenssh === 'string' &&
		typeof context.actualPublicKeyOpenssh === 'string'
	) {
		return {
			host: context.host,
			port: context.port,
			keyType: context.keyType,
			expectedFingerprintSha256: context.expectedFingerprintSha256,
			actualFingerprintSha256: context.actualFingerprintSha256,
			expectedPublicKeyOpenssh: context.expectedPublicKeyOpenssh,
			actualPublicKeyOpenssh: context.actualPublicKeyOpenssh
		};
	}

	return null;
}

