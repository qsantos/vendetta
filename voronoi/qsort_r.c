/*\
 *  Voronoi diagram by Fortune's algorithm
 *  Copyright (C) 2013-2014 Quentin SANTOS
 *
 *  This program is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  This program is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with this program.  If not, see <http://www.gnu.org/licenses/>.
\*/

#include "qsort_r.h"

#include <string.h>
#include <stdlib.h>

static void xchg(char *base, size_t size, size_t a, size_t b)
{
	char buffer[size];
	memcpy(buffer,      base+a*size, size);
	memcpy(base+a*size, base+b*size, size);
	memcpy(base+b*size, buffer,      size);
}

static void aux(char *base, size_t nmemb, size_t size,
        int (*compar)(const void *, const void *, void *), 
        void *arg, size_t a, size_t b)
{
	if (a >= b)
		return;
	if (b - a == 1)
	{
		if (compar(base+a*size, base+b*size, arg) > 0)
			xchg(base, size, a, b);
		return;
	}

	// choose pivot
	size_t p = a + (rand() % (b-a));

	// partition
	xchg(base, size, p, b);
	p = b;
	size_t s = a;
	for (size_t i = a; i < b; i++)
	{
		if (compar(base+i*size, base+p*size, arg) <= 0)
		{
			xchg(base, size, i, s);
			s++;
		}
	}
	xchg(base, size, s, p);

	// recursive sorting
	if (s > 0)
		aux(base, nmemb, size, compar, arg, a, s-1);
	aux(base, nmemb, size, compar, arg, s+1, b);
}

void qsort_r(void *base, size_t nmemb, size_t size,
        int (*compar)(const void *, const void *, void *), 
        void *arg)
{
	if (nmemb > 0)
		aux(base, nmemb, size, compar, arg, 0, nmemb-1);
}
