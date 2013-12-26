#include "transform.h"

#include <math.h>
#include <string.h>

#include "../util.h"
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

void transform_req(transform_t* t, int id, float a, char is_item)
{
	t->req = CREALLOC(t->req, component_t, t->n_req+1);
	t->req[t->n_req++] = (component_t){id, a, is_item, 0};
}

void transform_res(transform_t* t, int id, float a, char is_item)
{
	t->res = CREALLOC(t->res, component_t, t->n_res+1);
	t->res[t->n_res++] = (component_t){id, a, is_item, 0};
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

void transform_apply(transform_t* t, inventory_t* inv, float ratio)
{
	for (int i = 0; i < t->n_res; i++)
	{
		component_t* c = &t->res[i];
		if (c->kept)
			continue;

		if (c->is_item)
			inv->items[c->id] += ratio * c->amount;
		else
			inv->materials[c->id] += ratio * c->amount;
	}
}
