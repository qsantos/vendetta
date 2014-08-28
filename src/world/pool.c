/*\
 *  Role playing, management and strategy game
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

#include "pool.h"

#include <string.h>
#include <stdlib.h>

#include "../mem.h"

void pool_init(pool_t* p)
{
	p->n_objects = 0;
	p->a_objects = 0;
	p->objects = NULL;

	p->n_indices = 0;
	p->indices = NULL;
}

void pool_exit(pool_t* p)
{
	free(p->indices);
	for (size_t i = 0; i < p->n_objects; i++)
		free(p->objects[i]);
	free(p->objects);
}

object_t* pool_new(pool_t* p, uuid_t uuid, size_t size)
{
	if (pool_get(p, uuid) != NULL)
	{
		fprintf(stderr, "Trying to re-initate object %li\n", uuid);
		exit(1);
	}

	object_t* o = (object_t*) CALLOC(char, size);
	o->uuid = uuid >= 0 ? uuid : (uuid_t) p->n_indices;
	o->dead = 0;
	pool_push(p, o);
	return o;
}

object_t* pool_get(pool_t* p, uuid_t uuid)
{
	if (!(0 <= uuid && uuid < (ssize_t) p->n_indices))
		return NULL;

	ssize_t idx = p->indices[uuid];
	if (idx < 0)
		return NULL;

	object_t* o = p->objects[idx];
	if (o->uuid != uuid)
		return NULL;

	return o;
}

void pool_del(pool_t* p, object_t* a)
{
	(void) p;

	if (a == NULL)
		return;

	a->dead = 1;
}

void pool_upd(pool_t* p)
{
	size_t idx = 0;
	while (idx < p->n_objects)
	{
		object_t* o = p->objects[idx];
		if (o->dead)
		{
			pool_swap(p, idx, p->n_objects-1);
			pool_pop(p);
			free(o);
		}
		else
			idx++;
	}
}

void pool_push(pool_t* p, object_t* o)
{
	if (p->n_objects == p->a_objects)
	{
		p->a_objects = p->a_objects == 0 ? 1 : 2*p->a_objects;
		p->objects = CREALLOC(p->objects, object_t*, p->a_objects);
	}
	ssize_t idx = p->n_objects++;
	p->objects[idx] = o;

	// update indices
	if ((size_t) o->uuid >= p->n_indices)
	{
		size_t a_indices = o->uuid+1;
		p->indices = CREALLOC(p->indices, ssize_t, a_indices);
		for (size_t i = p->n_indices; i < a_indices; i++)
			p->indices[i] = -1;
		p->n_indices = a_indices;
	}
	p->indices[o->uuid] = idx;
}

void pool_pop(pool_t* p)
{
	p->n_objects--;

	// update indices
	p->indices[p->objects[p->n_objects]->uuid] = -1;
}

void pool_swap(pool_t* p, size_t a, size_t b)
{
	object_t* t = p->objects[a];
	p->objects[a] = p->objects[b];
	p->objects[b] = t;

	// update indices
	p->indices[p->objects[a]->uuid] = a;
	p->indices[p->objects[b]->uuid] = b;
}

#define SPECIALIZED(T,K) \
T##_t* T##_new(pool_t* p, uuid_t uuid) \
{ \
	return (T##_t*) pool_new(p, uuid, sizeof(T##_t)); \
} \
T##_t* T##_get(pool_t* p, uuid_t uuid) \
{ \
	T##_t* r = (T##_t*) pool_get(p, uuid); \
	if (r == NULL || r->o.t != K) return NULL; \
	return r; \
}

SPECIALIZED(projectile, O_PROJECTILE)
SPECIALIZED(character,  O_CHARACTER)
SPECIALIZED(mine,       O_MINE)
SPECIALIZED(building,   O_BUILDING)
