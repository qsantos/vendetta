#include "component.h"

#include <math.h>
#include <string.h>

#include "../util.h"
#include "../world/inventory.h"

void components_init(components_t* l)
{
	l->n = 0;
	l->c = 0;
	l->rate = 1;
}

void components_exit(components_t* l)
{
	free(l->c);
}

void components_copy(components_t* l, components_t* from)
{
	l->n = from->n;
	l->c = CALLOC(component_t, l->n);
	l->rate = from->rate;
	memcpy(l->c, from->c, l->n*sizeof(component_t));
}

void components_material(components_t* l, int id, float a)
{
	l->c = CREALLOC(l->c, component_t, l->n+1);
	l->c[l->n++] = (component_t){id, a, 0, 0};
}

void components_item(components_t* l, int id, int a)
{
	l->c = CREALLOC(l->c, component_t, l->n+1);
	l->c[l->n++] = (component_t){id, a, 1, 0};
}

int components_check(components_t* l, inventory_t* inv)
{
	for (int i = 0; i < l->n; i++)
	{
		component_t* c = &l->c[i];

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

float components_ratio(components_t* l, inventory_t* inv, float max_ratio)
{
	float ret = max_ratio;
	for (int i = 0; i < l->n; i++)
	{
		component_t* c = &l->c[i];

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

void components_apply(components_t* l, inventory_t* inv, float ratio)
{
	for (int i = 0; i < l->n; i++)
	{
		component_t* c = &l->c[i];
		if (l->c[i].kept)
			continue;

		if (c->is_item)
			inv->items[c->id] += ratio * c->amount;
		else
			inv->materials[c->id] += ratio * c->amount;
	}
}
