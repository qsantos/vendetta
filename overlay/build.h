#ifndef O_BUILD_H
#define O_BUILD_H

typedef struct ov_build ov_build_t;

#include "../universe/building.h"

struct ov_build
{
	kindOf_building_t* selected;

	// quick access
	char  active;
	float x;
	float y;
	float radius;
	int   count;
	int*  list;
};

#include "../game.h"

void ov_build_init(ov_build_t* o);
void ov_build_exit(ov_build_t* o);

char ov_build_cursor(ov_build_t* o, game_t* g, float x, float y);
void ov_build_draw  (ov_build_t* o, game_t* g);
char ov_build_catch (ov_build_t* o, game_t* g, float x, float y, float t);

#endif
