#include "component.h"

#include "../util.h"
#include "../world/inventory.h"

void components_init(components_t* l)
{
	l->n = 0;
	l->c = 0;
}

void components_exit(components_t* l)
{
	free(l->c);
}

void components_push(components_t* l, int id, float a)
{
	l->c = CREALLOC(l->c, component_t, l->n+1);
	l->c[l->n++] = (component_t){id, a, 0, 0};
}

int components_check(components_t* l, inventory_t* inv)
{
	for (int i = 0; i < l->n; i++)
	{
		float got  = inv->materials[l->c[i].id];
		float need = l->c[i].amount;
		if (got < need)
			return 0;
	}
	return 1;
}

float components_ratio(components_t* l, inventory_t* inv, float max_ratio)
{
	float ret = max_ratio;
	for (int i = 0; i < l->n; i++)
	{
		float got  = inv->materials[l->c[i].id];
		float need = l->c[i].amount;
		float r = got / need;
		if (ret < 0 || r < ret)
			ret = r;
	}
	return ret;
}

void components_apply(components_t* l, inventory_t* inv, float ratio)
{
	for (int i = 0; i < l->n; i++)
		if (!l->c[i].kept)
			inv->materials[l->c[i].id] += ratio * l->c[i].amount;
}
