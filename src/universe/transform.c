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

#include "transform.h"

#include <stdlib.h>
#include <math.h>
#include <string.h>

#include "../mem.h"
#include "../world/inventory.h"

void transform_init(transform_t* t)
{
	t->n_req = 0;
	t->req = NULL;

	t->n_res = 0;
	t->res = NULL;

	t->rate = 1;
}

void transform_exit(transform_t* t)
{
	free(t->res);
	free(t->req);
}

void transform_copy(transform_t* t, transform_t* from)
{
	t->n_req = from->n_req;
	t->req = CALLOC(component_t, t->n_req);
	memcpy(t->req, from->req, sizeof(component_t)*t->n_req);

	t->n_res = from->n_res;
	t->res = CALLOC(component_t, t->n_res);
	memcpy(t->res, from->res, sizeof(component_t)*t->n_res);

	t->rate = from->rate;
}

void transform_req(transform_t* t, char is_item, int id, float a)
{
	t->req = CREALLOC(t->req, component_t, t->n_req+1);
	t->req[t->n_req++] = (component_t){is_item, id, a};
}

void transform_res(transform_t* t, char is_item, int id, float a)
{
	t->res = CREALLOC(t->res, component_t, t->n_res+1);
	t->res[t->n_res++] = (component_t){is_item, id, a};
}

void transform_add(transform_t* t, transform_t* a, float r)
{
	for (int i = 0; i < a->n_req; i++)
	{
		component_t* c = &a->req[i];
		int j = transform_is_req(t, c->is_item, c->id);
		if (j >= 0)
			t->req[j].amount += c->amount * r;
		else
			transform_req(t, c->is_item, c->id, c->amount * r);
	}
	for (int i = 0; i < a->n_res; i++)
	{
		component_t* c = &a->res[i];
		int j = transform_is_res(t, c->is_item, c->id);
		if (j >= 0)
			t->res[j].amount += c->amount * r;
		else
			transform_res(t, c->is_item, c->id, c->amount * r);
	}
}

int transform_check(transform_t* t, inventory_t* inv)
{
	for (int i = 0; i < t->n_req; i++)
	{
		component_t* c = &t->req[i];

		float got;
		if (c->is_item)
			got = inv->items[c->id];
		else
			got = inv->materials[c->id];

		if (got < c->amount)
			return 0;
	}
	return 1;
}

float transform_ratio(transform_t* t, inventory_t* inv, float max_ratio)
{
	float ret = max_ratio;
	for (int i = 0; i < t->n_req; i++)
	{
		component_t* c = &t->req[i];

		float r;
		if (c->is_item)
			r = floor(inv->items[c->id] / c->amount);
		else
			r = inv->materials[c->id] / c->amount;

		if (ret < 0 || r < ret)
			ret = r;
	}
	return ret;
}

float transform_apply(transform_t* t, inventory_t* inv, float ratio)
{
	ratio = transform_ratio(t, inv, ratio);
	for (int i = 0; i < t->n_req; i++)
	{
		component_t* c = &t->req[i];
		if (c->is_item)
			inv->items[c->id] -= ratio * c->amount;
		else
			inv->materials[c->id] -= ratio * c->amount;
	}
	for (int i = 0; i < t->n_res; i++)
	{
		component_t* c = &t->res[i];
		if (c->is_item)
			inv->items[c->id] += ratio * c->amount;
		else
			inv->materials[c->id] += ratio * c->amount;
	}
	return ratio;
}

int transform_is_req(transform_t* t, char is_item, int id)
{
	for (int i = 0; i < t->n_req; i++)
	{
		component_t* c = &t->req[i];
		if (c->id == id && c->is_item == is_item)
			return i;
	}
	return -1;
}

int transform_is_res(transform_t* t, char is_item, int id)
{
	for (int i = 0; i < t->n_res; i++)
	{
		component_t* c = &t->res[i];
		if (c->id == id && c->is_item == is_item)
			return i;
	}
	return -1;
}

