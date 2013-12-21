#ifndef UTIL_H
#define UTIL_H

#include <stdlib.h>
#include <stdio.h>

inline void* check_alloc(size_t n, const char* file, int line)
{
	void* ret = malloc(n);
	if (ret == NULL)
	{
		fprintf(stderr, "Could not allocate memory at '%s' line %i\n", file, line);
		exit(1);
	}
	return ret;
}

#define CALLOC(T,N) ( (T*) check_alloc(N*sizeof(T), __FILE__, __LINE__) )

#endif
