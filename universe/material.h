#ifndef U_MATERIAL_H
#define U_MATERIAL_H

typedef struct kindOf_material kindOf_material_t;

#include <wchar.h>

#include "skill.h"
#include "status.h"

struct kindOf_material
{
	wchar_t* name;

	kindOf_skill_t skill;

	char edible;
	float eatBonus[N_STATUSES];
};

void kindOf_material_init(kindOf_material_t* m);
void kindOf_material_exit(kindOf_material_t* m);

#endif
