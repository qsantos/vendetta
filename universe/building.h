#ifndef U_BUILDING_H
#define U_BUILDING_H

typedef struct kindOf_building kindOf_building_t;

#include "../graphics.h"

struct kindOf_building
{
	const char* name;

	float width;
	float height;
	int   sprite;
};

void kindOf_building_init(kindOf_building_t* b, graphics_t* g, const char* name);
void kindOf_building_exit(kindOf_building_t* b);

#endif
