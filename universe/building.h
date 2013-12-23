#ifndef U_BUILDING_H
#define U_BUILDING_H

typedef struct kindOf_building kindOf_building_t;

struct kindOf_building
{
	const char* name;
	float       width;
	float       height;
	int         sprite;
};

#endif
