#include "material.h"

#include <stdlib.h>

void kindOf_material_init(kindOf_material_t* m)
{
	m->name = NULL;
	kindOf_skill_init(&m->skill);
	m->edible = 0;
	for (int i = 0; i < N_STATUSES; i++)
		m->eatBonus[i] = 0;
}

void kindOf_material_exit(kindOf_material_t* m)
{
	kindOf_skill_exit(&m->skill);
	free(m->name);
}
