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

#include "heap.h"

#include <stdlib.h>
#include <string.h>
#include <assert.h>

void heap_init(heap_t* h)
{
	h->size  = 0;
	h->avail = 0;
	h->tree  = NULL;
}

void heap_exit(heap_t* h)
{
	free(h->tree);
}

static inline int max(int a, int b)
{
	return a > b ? a : b;
}
static inline size_t parent(size_t i)
{
	return max(0, ((ssize_t)i+1)/2-1);
}
static inline size_t left(size_t i)
{
	return 2*i+1;
}
static inline size_t right(size_t i)
{
	return 2*i+2;
}
static inline void xchg(heap_t* h, size_t i, size_t j)
{
	hnode_t tmp = h->tree[i];
	h->tree[i] = h->tree[j];
	h->tree[j] = tmp;
}
static void bubbleUp(heap_t* h, size_t i)
{
	size_t p = parent(i);
	while (h->tree[i].idx < h->tree[p].idx)
	{
		xchg(h, i, p);
		i = p;
		p = parent(i);
	}
}
static void sinkDown(heap_t* h, size_t i)
{
	while (1)
	{
		size_t l = left(i);
		size_t r = right(i);

		size_t next = i;
		if (l < h->size && h->tree[l].idx < h->tree[next].idx)
			next = l;
		if (r < h->size && h->tree[r].idx < h->tree[next].idx)
			next = r;

		if (next != i)
		{
			xchg(h, i, next);
			i = next;
		}
		else
			break;
	}
}

void heap_insert(heap_t* h, double idx, void* data)
{
	if (h->size == h->avail)
	{
		h->avail = h->avail ? 2*h->avail : 1;
		h->tree = (hnode_t*) realloc(h->tree, sizeof(hnode_t)*h->avail);
		assert(h->tree != NULL);
	}

	size_t i = h->size++;
	h->tree[i] = (hnode_t){idx,data};
	bubbleUp(h, i);
}

void* heap_remove(heap_t* h)
{
	if (h->size == 0)
		return NULL;

	void* ret = h->tree[0].data;
	if (--h->size != 0)
	{
		xchg(h, 0, h->size);
		sinkDown(h, 0);
	}

	if (h->size < h->avail/4)
	{
		h->avail /= 2;
		h->tree = (hnode_t*) realloc(h->tree, sizeof(hnode_t)*h->avail);
		assert(h->tree != NULL);
	}
	return ret;
}
