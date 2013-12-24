#include "material.h"

#include <stdlib.h>

void kindOf_material_init(kindOf_material_t* m)
{
	m->name = NULL;
	kindOf_skill_init(&m->skill);
}

void kindOf_material_exit(kindOf_material_t* m)
{
	kindOf_skill_exit(&m->skill);
	free(m->name);
}
